use crate::AppError;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{self as proc, Command};
use std::sync::{
    atomic::{self, AtomicU32},
    Arc,
};
use std::{env, thread};
use tokio::sync::{mpsc, oneshot};
use tracing::{error, info};
use walkdir::WalkDir;

#[derive(Clone)]
pub struct FallbackDecoder {
    sender: mpsc::Sender<FDRequest>,
    current_child_pid: Arc<AtomicU32>,
}

struct FDRequest {
    cbor: Vec<u8>,
    response_tx: oneshot::Sender<Result<serde_json::Value, String>>,
}

impl FallbackDecoder {
    /// Starts a new child process.
    pub fn spawn() -> Result<Self, AppError> {
        let search_roots = [
            // Build script
            "target/testgen-hs/extracted/testgen-hs",
            // Common paths
            "/usr/bin",
            "/bin",
            // Docker
            "/app/testgen-hs",
        ];
        let testgen_hs_lib = Self::find_testgen_hs(&search_roots).map_err(AppError::Server)?;

        info!("Using {} as a fallback CBOR error decoder", &testgen_hs_lib);

        let current_child_pid = Arc::new(AtomicU32::new(u32::MAX));
        let current_child_pid_clone = current_child_pid.clone();
        let (sender, mut receiver) = mpsc::channel::<FDRequest>(128);

        // Clone `testgen_hs_path` for the thread.
        let testgen_hs_path_for_thread = testgen_hs_lib.clone();

        thread::spawn(move || {
            // For retries:
            let mut last_unfulfilled_request: Option<FDRequest> = None;

            loop {
                let single_run = Self::spawn_child(
                    &testgen_hs_path_for_thread,
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

        Ok(Self {
            sender,
            current_child_pid,
        })
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

    /// Searches first for `testgen-hs` in your project root, then recursively in the specified
    /// directories. Returns the first valid path found or an error if nothing is found.
    pub fn find_testgen_hs(roots: &[&str]) -> Result<String, String> {
        // 1. Check project root
        let manifest_dir = env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string());

        #[cfg_attr(not(target_os = "windows"), allow(unused_mut))]
        let mut root_candidate: PathBuf = PathBuf::from(&manifest_dir).join("testgen-hs");

        // On Windows, adjust the filename
        #[cfg(target_os = "windows")]
        {
            root_candidate.set_file_name("testgen-hs.exe");
        }

        // If the file exists and responds to --version, return it
        if Self::is_executable(root_candidate.as_path()) {
            return Ok(root_candidate.to_string_lossy().to_string());
        }

        // 2. Not found in project root, so recursively walk the directories in `roots`
        for root in roots {
            for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
                let path = entry.path();
                if path.is_file()
                    && path.file_name().unwrap_or_default() == "testgen-hs"
                    && Self::is_executable(path)
                {
                    return Ok(path.to_string_lossy().to_string());
                }
            }
        }

        Err("No valid `testgen-hs` binary found in subdomains.".to_string())
    }

    /// Checks if the path is runnable. Adjust for platform specifics if needed.
    fn is_executable(path: &Path) -> bool {
        Command::new(path).arg("--version").output().is_ok()
    }

    /// This function is called at startup, so that we make sure that the worker is reasonable.
    pub async fn startup_sanity_test(&self) -> Result<(), String> {
        let input = hex::decode("8182068182028200a0").map_err(|err| err.to_string())?;
        let result = self.decode(&input).await;
        let expected = serde_json::json!({
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
        });

        if result == Ok(expected) {
            Ok(())
        } else {
            Err(format!(
                "FallbackDecoder: startup_sanity_test failed: {:?}",
                result
            ))
        }
    }

    /// Returns the current child PID:
    pub fn child_pid(&self) -> Option<u32> {
        match self.current_child_pid.load(atomic::Ordering::Relaxed) {
            u32::MAX => None,
            pid => Some(pid),
        }
    }

    fn spawn_child(
        testgen_hs_path: &str,
        receiver: &mut mpsc::Receiver<FDRequest>,
        last_unfulfilled_request: &mut Option<FDRequest>,
        current_child_pid: &Arc<AtomicU32>,
    ) -> Result<(), String> {
        let mut child = proc::Command::new(testgen_hs_path)
            .arg("deserialize-stream")
            .stdin(proc::Stdio::piped())
            .stdout(proc::Stdio::piped())
            .spawn()
            .map_err(|err| format!("couldn’t start the child: {:?}", err))?;

        current_child_pid.store(child.id(), atomic::Ordering::Relaxed);

        let result = Self::process_requests(&mut child, receiver, last_unfulfilled_request);

        // Let’s make sure it’s dead in case a different error landed us here.
        // Will return Ok(()) if already dead.
        child
            .kill()
            .map_err(|err| format!("couldn’t kill the child: {:?}", err))?;
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

        while let Some((request, is_a_retry)) = last_unfulfilled_request
            .take()
            .map(|a| (a, true))
            .or_else(|| receiver.blocking_recv().map(|a| (a, false)))
        {
            let cbor_hex = hex::encode(&request.cbor);
            *last_unfulfilled_request = Some(request);

            let mut ask_and_receive = || -> Result<Result<serde_json::Value, String>, String> {
                writeln!(stdin, "{}", cbor_hex)
                    .map_err(|err| format!("couldn’t write to stdin: {:?}", err))?;

                match stdout_lines.next() {
                    Some(Ok(line)) => Ok(Self::parse_json(&line)),
                    Some(Err(e)) => Err(format!("failed to read from subprocess: {}", e)),
                    None => Err("no output from subprocess".to_string()),
                }
            };

            // Split the result to satisfy the borrow checker:
            let (result_for_response, result_for_logs) = partition_result(ask_and_receive());

            // We want to respond to the user with a failure in case this was a retry.
            // Otherwise, it’s an infinite loop and wait time for the response.
            if is_a_retry || result_for_response.is_ok() {
                // unwrap is safe, we wrote there right before the writeln!()
                let request = last_unfulfilled_request.take().unwrap();

                let response = match result_for_response {
                    Ok(ok) => ok,
                    Err(_) => Err("repeated internal failure".to_string()),
                };

                // unwrap is safe, the other side would have to drop for a
                // panic – can’t happen, because we control it:
                request.response_tx.send(response).unwrap();
            }

            // Now break the loop, and restart everything if we failed:
            result_for_logs?
        }

        Err("request channel closed, won’t happen".to_string())
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

fn partition_result<A, E>(ae: Result<A, E>) -> (Result<A, ()>, Result<(), E>) {
    match ae {
        Err(err) => (Err(()), Err(err)),
        Ok(ok) => (Ok(ok), Ok(())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[tracing_test::traced_test]
    async fn test_fallback_decoder() {
        let decoder = FallbackDecoder::spawn().unwrap();

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
