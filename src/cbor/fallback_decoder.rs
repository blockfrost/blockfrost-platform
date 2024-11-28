use std::io::{BufRead, BufReader, Write};
use std::process as proc;
use std::sync::{
    atomic::{self, AtomicU32},
    Arc,
};
use std::thread;
use tokio::sync::{mpsc, oneshot};
use tracing::error;

#[derive(Clone)]
pub struct FallbackDecoder {
    sender: mpsc::Sender<FDRequest>,
    current_child_pid: Arc<AtomicU32>,
}

struct FDRequest {
    cbor: Vec<u8>,
    response_tx: oneshot::Sender<Result<serde_json::Value, String>>,
}

const CHILD_EXE_NAME: &str = "testgen-hs";

impl FallbackDecoder {
    /// Starts a new child process.
    pub fn spawn() -> Self {
        let current_child_pid = Arc::new(AtomicU32::new(u32::MAX));
        let current_child_pid_clone = current_child_pid.clone();
        let (sender, mut receiver) = mpsc::channel::<FDRequest>(128);

        thread::spawn(move || {
            // For retries:
            let mut last_unfulfilled_request: Option<FDRequest> = None;

            loop {
                let single_run = Self::spawn_child(
                    &mut receiver,
                    &mut last_unfulfilled_request,
                    &current_child_pid_clone,
                );
                let restart_delay = std::time::Duration::from_secs(1);
                error!(
                    "FallbackDecoder: will restart in {:?} because of a subprocess error: {:?}",
                    restart_delay, single_run
                );
                std::thread::sleep(restart_delay);
            }
        });

        Self {
            sender,
            current_child_pid,
        }
    }

    /// Decodes a CBOR error using the child process.
    pub async fn decode(&self, cbor: &[u8]) -> Result<serde_json::Value, String> {
        let (response_tx, response_rx) = oneshot::channel();
        self.sender
            .send(FDRequest {
                cbor: cbor.to_vec(),
                response_tx,
            })
            .await
            .map_err(|err| format!("FallbackDecoder: failed to send request: {:?}", err))?;

        response_rx.await.map_err(|err| {
            format!(
                "FallbackDecoder: worker thread dropped (won’t happen): {:?}",
                err
            )
        })?
    }

    /// A heuristic to find the child binary that we’ll use.
    pub fn locate_child_binary() -> Result<String, String> {
        use std::env;

        let binary_name = if cfg!(windows) {
            format!("{}.exe", CHILD_EXE_NAME)
        } else {
            CHILD_EXE_NAME.to_string()
        };

        // Check in the CHILD_EXE_NAME subdirectory in the directory of the current binary:
        if let Ok(current_exe) = env::current_exe() {
            if let Some(current_dir) = current_exe.parent() {
                let potential_path = current_dir.join(CHILD_EXE_NAME).join(&binary_name);
                if potential_path.is_file() {
                    return Ok(potential_path.to_string_lossy().into_owned());
                }
            }
        }

        // Check PATH:
        if let Ok(paths) = env::var("PATH") {
            for path in env::split_paths(&paths) {
                let potential_path = path.join(&binary_name);
                if potential_path.is_file() {
                    return Ok(potential_path.to_string_lossy().into_owned());
                }
            }
        }

        // Check CHILD_EXE_NAME/CHILD_EXE_NAME.exe in the current directory if
        // running tests and it contains Cargo.toml:
        if cfg!(test) {
            if let Ok(current_dir) = env::current_dir() {
                if current_dir.join("Cargo.toml").is_file() {
                    let potential_path = current_dir.join(CHILD_EXE_NAME).join(&binary_name);
                    if potential_path.is_file() {
                        return Ok(potential_path.to_string_lossy().into_owned());
                    }
                }
            }
        }

        Err(format!(
            "Could not find binary '{}' in the current directory or on PATH",
            binary_name
        ))
    }

    /// Returns the current child PID:
    pub fn child_pid(&self) -> Option<u32> {
        match self.current_child_pid.load(atomic::Ordering::Relaxed) {
            u32::MAX => None,
            pid => Some(pid),
        }
    }

    fn spawn_child(
        receiver: &mut mpsc::Receiver<FDRequest>,
        last_unfulfilled_request: &mut Option<FDRequest>,
        current_child_pid: &Arc<AtomicU32>,
    ) -> Result<(), String> {
        // FIXME: _find_ the exe_path
        // FIXME: make a release with LineBuffering
        let exe_path = Self::locate_child_binary().unwrap_or(CHILD_EXE_NAME.to_string());

        let mut child = proc::Command::new(exe_path)
            .arg("deserialize-stream")
            .stdin(proc::Stdio::piped())
            .stdout(proc::Stdio::piped())
            .spawn()
            .map_err(|err| format!("couldn’t start the child: {:?}", err))?;

        current_child_pid.store(child.id(), atomic::Ordering::Relaxed);

        let result = Self::process_requests(&mut child, receiver, last_unfulfilled_request);

        child
            .wait()
            .map_err(|err| format!("couldn’t reap the child: {:?}", err))?;

        result
    }

    fn process_requests(
        child: &mut proc::Child,
        receiver: &mut mpsc::Receiver<FDRequest>,
        last_unfulfilled_request: &mut Option<FDRequest>,
    ) -> Result<(), String> {
        let stdin = child
            .stdin
            .as_mut()
            .ok_or("couldn’t grab stdin".to_string())?;
        let stdout = child
            .stdout
            .as_mut()
            .ok_or("couldn’t grab stdout".to_string())?;
        let stdout_reader = BufReader::new(stdout);
        let mut stdout_lines = stdout_reader.lines();

        while let Some(request) = last_unfulfilled_request
            .take()
            .or_else(|| receiver.blocking_recv())
        {
            let cbor_hex = hex::encode(&request.cbor);
            *last_unfulfilled_request = Some(request);

            writeln!(stdin, "{}", cbor_hex)
                .map_err(|err| format!("couldn’t write to stdin: {:?}", err))?;

            let result: Result<serde_json::Value, String> = match stdout_lines.next() {
                Some(Ok(line)) => Self::parse_json(&line),
                Some(Err(e)) => Err(format!("failed to read from subprocess: {}", e))?,
                None => Err("no output from subprocess".to_string())?,
            };

            // unwrap is safe, we wrote there right before the writeln!()
            let request = last_unfulfilled_request.take().unwrap();

            // unwrap is safe, the other side would have to drop for a
            // panic – can’t happen, because we control it:
            request.response_tx.send(result).unwrap();
        }

        Err("reached EOF".to_string())
    }

    fn parse_json(input: &str) -> Result<serde_json::Value, String> {
        let mut parsed: serde_json::Value =
            serde_json::from_str(input).map_err(|e| e.to_string())?;

        parsed
            .as_object()
            .and_then(|obj| {
                if obj.len() == 1 {
                    obj.get("error")
                        .and_then(|v| v.as_str())
                        .map(|s| Err(s.to_string()))
                } else {
                    None
                }
            })
            .unwrap_or_else(|| {
                parsed
                    .get_mut("json")
                    .map(serde_json::Value::take)
                    .ok_or_else(|| "Missing 'json' field".to_string())
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_deserialization() {
        FallbackDecoder::locate_child_binary().unwrap();

        let decoder = FallbackDecoder::spawn();
        let input = hex::decode("8182068182028200a0").unwrap();
        let result = decoder.decode(&input).await;
        assert_eq!(
            result,
            Ok(serde_json::json!({
              "contents": {
                "contents": {
                  "contents": {
                    "era": "ShelleyBasedEraConway",
                    "error": [
                      "ConwayCertsFailure (WithdrawalsNotInRewardsCERTS (fromList []))"
                    ],
                    "kind": "ShelleyTxValidationError"
                  },
                  "tag": "TxValidationErrorInCardanoMode"
                },
                "tag": "TxCmdTxSubmitValidationError"
              },
              "tag": "TxSubmitFail"
            }))
        );

        // Now, kill our child to test the restart logic:
        sysinfo::System::new_all()
            .process(sysinfo::Pid::from_u32(decoder.child_pid().unwrap()))
            .unwrap()
            .kill();

        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        let input = hex::decode("8182068183051a000c275b1a000b35ec").unwrap();
        let result = decoder.decode(&input).await;
        assert_eq!(
            result,
            Ok(serde_json::json!({
              "contents": {
                "contents": {
                  "contents": {
                    "era": "ShelleyBasedEraConway",
                    "error": [
                      "ConwayTreasuryValueMismatch (Coin 796507) (Coin 734700)"
                    ],
                    "kind": "ShelleyTxValidationError"
                  },
                  "tag": "TxValidationErrorInCardanoMode"
                },
                "tag": "TxCmdTxSubmitValidationError"
              },
              "tag": "TxSubmitFail"
            }))
        );
    }
}
