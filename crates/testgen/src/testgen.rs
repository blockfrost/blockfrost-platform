use bf_common::errors::AppError;
use serde::Deserialize;
use serde::de;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{self as proc, Command};
use std::sync::{
    Arc,
    atomic::{self, AtomicU32},
};
use std::{env, thread};
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, warn};

/// Handle to a long-running `testgen-hs` subprocess. Cheap to clone;
/// clones share one worker thread.
#[derive(Clone)]
pub struct Testgen {
    sender: mpsc::Sender<TestgenRequest>,
    current_child_pid: Arc<AtomicU32>,
}

struct TestgenRequest {
    payload: String,
    response: oneshot::Sender<Result<TestgenResponse, String>>,
}

/// Which `testgen-hs` subcommand to run.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub enum Variant {
    DeserializeStream,
}

impl Variant {
    fn as_arg(self) -> &'static str {
        match self {
            Self::DeserializeStream => "deserialize-stream",
        }
    }
}

impl std::fmt::Display for Variant {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_arg())
    }
}

#[derive(Debug, PartialEq)]
pub enum TestgenResponse {
    Ok(serde_json::Value),
    Err(serde_json::Value),
}

const MISSING_BOTH_FIELDS_MSG: &str =
    "invalid testgen-hs response: missing both `json` and `error`";

const REQUEST_CHANNEL_CAPACITY: usize = 128;
const RESTART_DELAY: std::time::Duration = std::time::Duration::from_secs(1);
const RESPONSE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

#[derive(Deserialize)]
struct TestgenResponseWire {
    #[serde(default)]
    json: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<serde_json::Value>,
}

impl<'de> Deserialize<'de> for TestgenResponse {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let wire = TestgenResponseWire::deserialize(deserializer)?;
        match (wire.json, wire.error) {
            (Some(json), Some(err)) => {
                warn!(
                    "testgen-hs response has both `json` and `error` fields, discarding error: {}",
                    err
                );
                Ok(Self::Ok(json))
            },
            (Some(json), None) => Ok(Self::Ok(json)),
            (None, Some(error)) => Ok(Self::Err(error)),
            (None, None) => Err(de::Error::custom(MISSING_BOTH_FIELDS_MSG)),
        }
    }
}

impl Testgen {
    /// Starts a new child process.
    pub fn spawn(variant: Variant) -> Result<Self, AppError> {
        Self::spawn_inner(variant, None)
    }

    /// Starts a new child process with an init payload that is sent as the first
    /// message on every (re)start. The response to the init payload is validated
    /// but not returned — it is only used to confirm the subprocess is ready.
    pub fn spawn_with_init(variant: Variant, init_payload: String) -> Result<Self, AppError> {
        Self::spawn_inner(variant, Some(init_payload))
    }

    fn spawn_inner(variant: Variant, init_payload: Option<String>) -> Result<Self, AppError> {
        let testgen_hs_path = Self::find_testgen_hs().map_err(AppError::Server)?;

        info!(
            "Spawning testgen-hs ({}) from {}",
            variant, &testgen_hs_path
        );

        let current_child_pid = Arc::new(AtomicU32::new(u32::MAX));
        let current_child_pid_clone = current_child_pid.clone();
        let (sender, mut receiver) = mpsc::channel::<TestgenRequest>(REQUEST_CHANNEL_CAPACITY);
        let testgen_hs_path_for_thread = testgen_hs_path.clone();

        thread::spawn(move || {
            let mut last_unfulfilled_request: Option<TestgenRequest> = None;

            loop {
                let single_run = Self::spawn_child(
                    &testgen_hs_path_for_thread,
                    &mut receiver,
                    &mut last_unfulfilled_request,
                    &current_child_pid_clone,
                    variant,
                    init_payload.as_deref(),
                );

                // Exit if no work is pending and all senders are gone.
                if last_unfulfilled_request.is_none() {
                    match receiver.try_recv() {
                        Err(mpsc::error::TryRecvError::Disconnected) => {
                            info!("Testgen: all senders dropped, shutting down background worker");
                            break;
                        },
                        Ok(req) => {
                            last_unfulfilled_request = Some(req);
                        },
                        Err(mpsc::error::TryRecvError::Empty) => {},
                    }
                }

                error!(
                    "Testgen: will restart in {RESTART_DELAY:?} because of a subprocess error: {}",
                    single_run.err().unwrap_or_else(|| "unknown".to_string())
                );
                std::thread::sleep(RESTART_DELAY);

                // Re-check after the delay: senders may have dropped while we slept.
                if last_unfulfilled_request.is_none() && receiver.is_closed() {
                    info!("Testgen: all senders dropped during restart delay, shutting down");
                    break;
                }
            }
        });

        Ok(Self {
            sender,
            current_child_pid,
        })
    }

    /// Sends the payload to the child process.
    pub async fn decode(&self, cbor: &[u8]) -> Result<TestgenResponse, String> {
        self.send(hex::encode(cbor)).await
    }

    /// Sends the payload to the child process.
    pub async fn send(&self, payload: String) -> Result<TestgenResponse, String> {
        let (response, response_rx) = oneshot::channel();

        self.sender
            .send(TestgenRequest { payload, response })
            .await
            .map_err(|err| format!("Testgen: failed to send request: {err:?}"))?;

        tokio::time::timeout(RESPONSE_TIMEOUT, response_rx)
            .await
            .map_err(|_| format!("Testgen: request timed out after {RESPONSE_TIMEOUT:?}"))?
            .map_err(|err| format!("Testgen: worker thread dropped: {err:?}"))?
    }

    /// Searches for `testgen-hs` in multiple directories.
    pub fn find_testgen_hs() -> Result<String, String> {
        let exe_name = if cfg!(target_os = "windows") {
            "testgen-hs.exe"
        } else {
            "testgen-hs"
        };

        let mut search_paths: Vec<PathBuf> = Vec::new();

        if let Ok(path) = env::var("TESTGEN_HS_PATH") {
            search_paths.push(PathBuf::from(path));
        }

        if let Some(path) = option_env!("TESTGEN_HS_PATH") {
            search_paths.push(PathBuf::from(path));
        }

        // This is the most important one for relocatable directories (that keep the initial
        // structure) on Windows, Linux, macOS.
        if let Ok(current_exe) = env::current_exe()
            && let Ok(current_exe) = std::fs::canonicalize(current_exe)
            && let Some(exe_dir) = current_exe.parent()
        {
            search_paths.push(exe_dir.join(exe_name));

            // build_utils::testgen_hs::ensure extracts to target/{debug|release}/testgen-hs/.
            search_paths.push(exe_dir.join("testgen-hs").join(exe_name));

            if let Some(profile_dir) = exe_dir.parent() {
                search_paths.push(profile_dir.join("testgen-hs").join(exe_name));
            }
        }

        // Docker image fallback.
        search_paths.push(PathBuf::from("/app/testgen-hs"));

        // System PATH lookup.
        search_paths.extend(
            env::var("PATH")
                .map(|p| {
                    env::split_paths(&p)
                        .map(|dir| dir.join(exe_name))
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default(),
        );

        debug!("{} search paths = {:?}", exe_name, search_paths);

        // Checks if the path is runnable. Adjust for platform specifics if needed.
        // TODO: check that the --version matches what we expect.
        fn is_our_executable(path: &Path) -> bool {
            Command::new(path).arg("--version").output().is_ok()
        }

        // Look in each candidate path to find a matching executable.
        for candidate in &search_paths {
            let path = if candidate.file_name().is_some_and(|name| name == exe_name) {
                candidate.clone()
            } else {
                candidate.join(exe_name)
            };

            if path.is_file() && is_our_executable(path.as_path()) {
                return Ok(path.to_string_lossy().to_string());
            }
        }

        Err(format!(
            "No valid `{}` binary found in {:?}.",
            exe_name, &search_paths
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
        testgen_hs_path: &str,
        receiver: &mut mpsc::Receiver<TestgenRequest>,
        last_unfulfilled_request: &mut Option<TestgenRequest>,
        current_child_pid: &Arc<AtomicU32>,
        variant: Variant,
        init_payload: Option<&str>,
    ) -> Result<(), String> {
        let mut child = proc::Command::new(testgen_hs_path)
            .arg(variant.as_arg())
            .stdin(proc::Stdio::piped())
            .stdout(proc::Stdio::piped())
            .spawn()
            .map_err(|err| format!("couldn’t start the child: {err:?}"))?;

        current_child_pid.store(child.id(), atomic::Ordering::Relaxed);

        let result = Self::process_requests(
            &mut child,
            receiver,
            last_unfulfilled_request,
            init_payload,
            variant,
        );

        // Make sure the child is dead before returning.
        child
            .kill()
            .map_err(|err| format!("couldn’t kill the child: {err:?}"))?;
        child
            .wait()
            .map_err(|err| format!("couldn’t reap the child: {err:?}"))?;

        result
    }

    fn process_requests(
        child: &mut proc::Child,
        receiver: &mut mpsc::Receiver<TestgenRequest>,
        last_unfulfilled_request: &mut Option<TestgenRequest>,
        init_payload: Option<&str>,
        variant: Variant,
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

        // Send init payload before processing any requests.
        if let Some(payload) = init_payload {
            writeln!(stdin, "{payload}")
                .map_err(|err| format!("couldn’t write init payload: {err:?}"))?;
            match stdout_lines.next() {
                Some(Ok(line)) => {
                    let resp: TestgenResponse = serde_json::from_str(&line)
                        .map_err(|e| format!("init response parse error: {e}"))?;
                    match resp {
                        TestgenResponse::Ok(_) => {
                            info!("subprocess ({variant}) initialized successfully")
                        },
                        TestgenResponse::Err(e) => return Err(format!("init failed: {e}")),
                    }
                },
                Some(Err(e)) => return Err(format!("init read error: {e}")),
                None => return Err("no init response from subprocess".to_string()),
            }
        }

        while let Some((request, is_a_retry)) = last_unfulfilled_request
            .take()
            .map(|a| (a, true))
            .or_else(|| receiver.blocking_recv().map(|a| (a, false)))
        {
            let payload = request.payload.clone();
            *last_unfulfilled_request = Some(request);

            let mut ask_and_receive = || -> Result<Result<TestgenResponse, String>, String> {
                writeln!(stdin, "{payload}")
                    .map_err(|err| format!("couldn’t write to stdin: {err:?}"))?;

                match stdout_lines.next() {
                    Some(Ok(line)) => Ok(Ok(serde_json::from_str::<TestgenResponse>(&line)
                        .map_err(|e| e.to_string())?)),

                    Some(Err(e)) => Err(format!("failed to read from subprocess: {e}")),
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

                let _ = request.response.send(response);
            }

            // Now break the loop, and restart everything if we failed:
            result_for_logs?
        }

        Err("request channel closed".to_string())
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
    use super::{MISSING_BOTH_FIELDS_MSG, TestgenResponse};
    use serde_json::json;

    fn parse(s: &str) -> Result<TestgenResponse, serde_json::Error> {
        serde_json::from_str(s)
    }

    #[test]
    fn only_json_field_is_ok() {
        assert_eq!(
            parse(r#"{"json": {"value": 42}}"#).unwrap(),
            TestgenResponse::Ok(json!({"value": 42})),
        );
    }

    #[test]
    fn only_error_field_is_err() {
        assert_eq!(
            parse(r#"{"error": "boom"}"#).unwrap(),
            TestgenResponse::Err(json!("boom")),
        );
    }

    #[test]
    fn both_fields_prefer_json() {
        assert_eq!(
            parse(r#"{"json": {"ok": true}, "error": "ignored"}"#).unwrap(),
            TestgenResponse::Ok(json!({"ok": true})),
        );
    }

    #[test]
    fn neither_field_fails_deserialization() {
        let err = parse(r#"{}"#).unwrap_err();
        assert!(
            err.to_string().contains(MISSING_BOTH_FIELDS_MSG),
            "unexpected error: {err}",
        );
    }

    /// Locks in the wire contract: unknown fields are silently dropped, so
    /// `testgen-hs` can add new response fields without breaking us.
    #[test]
    fn unknown_fields_are_ignored() {
        assert_eq!(
            parse(r#"{"json": 1, "extra": "meta"}"#).unwrap(),
            TestgenResponse::Ok(json!(1)),
        );
    }
}
