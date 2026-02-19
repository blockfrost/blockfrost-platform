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
use tracing::{debug, error, info};

#[derive(Clone)]
pub struct Testgen {
    sender: mpsc::Sender<TestgenRequest>,
    pub current_child_pid: Arc<AtomicU32>,
}

pub struct TestgenRequest {
    payload: String,
    response: oneshot::Sender<Result<TestgenResponse, String>>,
}

#[derive(Debug)]
pub enum TestgenResponse {
    Ok(serde_json::Value),
    Err(serde_json::Value),
}

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
            (Some(json), _) => Ok(Self::Ok(json)),
            (None, Some(error)) => Ok(Self::Err(error)),
            (None, None) => Err(de::Error::custom(
                "invalid testgen-hs response: missing both `json` and `error`",
            )),
        }
    }
}
/// Testgen is an executable that we use to run some functionality that are readily/easily available
/// in Haskell codebase like the ledger.
/// The name is 'testgen' since it was initially implemented to generate test cases.
impl Testgen {
    /// Starts a new child process.
    pub fn spawn(variant: &str) -> Result<Self, AppError> {
        let testgen_hs_path = Self::find_testgen_hs().map_err(AppError::Server)?;

        info!(
            "Using {} as a fallback CBOR error decoder",
            &testgen_hs_path
        );

        let current_child_pid = Arc::new(AtomicU32::new(u32::MAX));
        let current_child_pid_clone = current_child_pid.clone();
        let variant_clone = variant.to_string();
        let (sender, mut receiver) = mpsc::channel::<TestgenRequest>(128);

        // Clone `testgen_hs_path` for the thread.
        let testgen_hs_path_for_thread = testgen_hs_path.clone();

        thread::spawn(move || {
            // For retries:
            let mut last_unfulfilled_request: Option<TestgenRequest> = None;

            loop {
                let single_run = Self::spawn_child(
                    &testgen_hs_path_for_thread,
                    &mut receiver,
                    &mut last_unfulfilled_request,
                    &current_child_pid_clone,
                    &variant_clone,
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
            .map_err(|err| format!("FallbackDecoder: failed to send request: {err:?}"))?;

        response_rx.await.unwrap_or_else(|err| {
            unreachable!(
                "FallbackDecoder: worker thread dropped (can’t happen): {:?}",
                err
            )
        })
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
        if let Ok(current_exe) = env::current_exe() {
            if let Ok(current_exe) = std::fs::canonicalize(current_exe) {
                if let Some(exe_dir) = current_exe.parent() {
                    search_paths.push(exe_dir.join(exe_name));

                    // build_utils::testgen_hs::ensure extracts to target/{debug|release}/testgen-hs.
                    if let Some(profile_dir) = exe_dir.parent() {
                        search_paths.push(profile_dir.join("testgen-hs").join(exe_name));
                    }
                }
            }
        }

        let target_dir_from_env = env::var("CARGO_TARGET_DIR").unwrap_or_else(|_| "target".into());

        // Runtime CARGO_MANIFEST_DIR can point to the current package (e.g. error_decoder tests).
        if let Ok(root) = env::var("CARGO_MANIFEST_DIR") {
            let target_dir = PathBuf::from(root).join(&target_dir_from_env);
            search_paths.push(target_dir.join("debug").join("testgen-hs").join(exe_name));
            search_paths.push(target_dir.join("release").join("testgen-hs").join(exe_name));
            search_paths.push(
                target_dir
                    .join("testgen-hs")
                    .join("extracted")
                    .join(exe_name),
            );
        }

        // Compile-time CARGO_MANIFEST_DIR always points to this crate.
        let target_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(target_dir_from_env);
        search_paths.push(target_dir.join("debug").join("testgen-hs").join(exe_name));
        search_paths.push(target_dir.join("release").join("testgen-hs").join(exe_name));
        search_paths.push(
            target_dir
                .join("testgen-hs")
                .join("extracted")
                .join(exe_name),
        );

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
        variant: &str,
    ) -> Result<(), String> {
        let mut child = proc::Command::new(testgen_hs_path)
            .arg(variant)
            .stdin(proc::Stdio::piped())
            .stdout(proc::Stdio::piped())
            .spawn()
            .map_err(|err| format!("couldn’t start the child: {err:?}"))?;

        current_child_pid.store(child.id(), atomic::Ordering::Relaxed);

        let result = Self::process_requests(&mut child, receiver, last_unfulfilled_request);

        // Let’s make sure it’s dead in case a different error landed us here.
        // Will return Ok(()) if already dead.
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

                // unwrap is safe, the other side would have to drop for a
                // panic – can’t happen, because we control it:
                request
                    .response
                    .send(response)
                    .unwrap_or_else(|_| unreachable!());
            }

            // Now break the loop, and restart everything if we failed:
            result_for_logs?
        }

        Err("request channel closed, won’t happen".to_string())
    }
}

fn partition_result<A, E>(ae: Result<A, E>) -> (Result<A, ()>, Result<(), E>) {
    match ae {
        Err(err) => (Err(()), Err(err)),
        Ok(ok) => (Ok(ok), Ok(())),
    }
}
