use std::io::{BufRead, BufReader, Write};
use std::process as proc;
use std::thread;
use tokio::sync::{mpsc, oneshot};
use tracing::error;

#[derive(Clone)]
pub struct FallbackDecoder {
    sender: mpsc::Sender<FDRequest>,
}

struct FDRequest {
    cbor: Vec<u8>,
    response_tx: oneshot::Sender<Result<serde_json::Value, String>>,
}

impl FallbackDecoder {
    /// Starts a new child process.
    pub fn spawn() -> Self {
        let (sender, mut receiver) = mpsc::channel::<FDRequest>(128);

        thread::spawn(move || {
            // For retries:
            let mut last_unfulfilled_request: Option<FDRequest> = None;

            loop {
                let single_run = Self::behavior(&mut receiver, &mut last_unfulfilled_request);
                let restart_delay = std::time::Duration::from_secs(1);
                error!(
                    "FallbackDecoder: will restart in {:?} because of a subprocess error: {:?}",
                    restart_delay, single_run
                );
                std::thread::sleep(restart_delay);
            }
        });

        Self { sender }
    }

    fn behavior(
        receiver: &mut mpsc::Receiver<FDRequest>,
        last_unfulfilled_request: &mut Option<FDRequest>,
    ) -> Result<(), String> {
        // FIXME: _find_ the exe_path
        // FIXME: make a release with LineBuffering
        let exe_path = "/nix/store/4y2jqhw3c2i407m8rmkvlja9wdr1kqhq-testgen-hs-exe-testgen-hs-x86_64-unknown-linux-musl-10.1.2.1/bin/testgen-hs";

        let child = proc::Command::new(exe_path)
            .arg("deserialize-stream")
            .stdin(proc::Stdio::piped())
            .stdout(proc::Stdio::piped())
            .spawn()
            .map_err(|err| format!("couldn’t start the child: {:?}", err))?;

        let mut stdin = child.stdin.ok_or("couldn’t grab stdin".to_string())?;
        let stdout = child.stdout.ok_or("couldn’t grab stdout".to_string())?;
        let stdout_reader = BufReader::new(stdout);
        let mut stdout_lines = stdout_reader.lines();

        while let Some(request) = last_unfulfilled_request.take().or(receiver.blocking_recv()) {
            let cbor_hex = hex::encode(&request.cbor);
            *last_unfulfilled_request = Some(request);

            writeln!(stdin, "{}", cbor_hex)
                .map_err(|err| format!("couldn’t write to stdin: {:?}", err))?;

            let result: Result<serde_json::Value, String> = match stdout_lines.next() {
                Some(Ok(line)) => Self::parse_json(&line),
                Some(Err(e)) => Err(format!("failed to read from subprocess: {}", e))?,
                None => Err("no output from subprocess".to_string())?,
            };

            let request = last_unfulfilled_request.take().unwrap();

            // unwrap is safe, the other side would have to drop for a
            // panic – can’t happen:
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
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn test_deserialization() {
        let wrapper = FallbackDecoder::spawn();
        let input = hex::decode("8182068182028200a0").unwrap();
        let result = wrapper.decode(&input).await;
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

        tokio::time::sleep(tokio::time::Duration::from_secs(5)).await;

        // FIXME: zombie process and hangs, when we kill it now

        let input = hex::decode("8182068183051a000c275b1a000b35ec").unwrap();
        let result = wrapper.decode(&input).await;
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
