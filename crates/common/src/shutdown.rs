use std::time::Duration;

use tracing::info;

/// How long graceful shutdown may take before we force-exit.
pub const GRACE_PERIOD: Duration = Duration::from_secs(10);

/// Resolves on the first SIGINT (Ctrl-C) or, on Unix, SIGTERM.
///
/// Handling SIGTERM matters for `docker stop`, systemd, and our test scripts:
/// its default disposition kills the process without running `atexit`
/// handlers, which – among other things – loses the profiles written by
/// `-Cinstrument-coverage` builds.
///
/// Once a signal arrives, a watchdog thread force-exits the process if
/// graceful shutdown doesn’t finish within [`GRACE_PERIOD`]. It uses
/// [`std::process::exit`] with status 0. Thus, the `atexit` handlers also run.
/// Graceful shutdown drains only the HTTP requests. When the program drops the
/// Tokio runtime, the runtime cancels the background tasks.
pub async fn signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};
        use tracing::warn;

        match signal(SignalKind::terminate()) {
            Ok(mut sigterm) => {
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => info!("Received SIGINT, shutting down"),
                    _ = sigterm.recv() => info!("Received SIGTERM, shutting down"),
                }
            },
            Err(err) => {
                warn!("Failed to install SIGTERM handler: {err}");
                let _ = tokio::signal::ctrl_c().await;
                info!("Received SIGINT, shutting down");
            },
        }
    }

    #[cfg(not(unix))]
    {
        let _ = tokio::signal::ctrl_c().await;
        info!("Received Ctrl-C, shutting down");
    }

    spawn_watchdog(GRACE_PERIOD);
}

// A plain thread, so that it survives the Tokio runtime being dropped:
fn spawn_watchdog(grace_period: Duration) {
    std::thread::spawn(move || {
        std::thread::sleep(grace_period);
        eprintln!(
            "Graceful shutdown didn’t finish within {}s, exiting",
            grace_period.as_secs()
        );
        std::process::exit(0);
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Stdio;
    use tokio::process::Command;

    const WATCHDOG_CHILD_ENV: &str = "BF_SHUTDOWN_TEST_WATCHDOG_CHILD";

    #[tokio::test]
    async fn watchdog_exits_zero_after_grace_period() {
        let exe = std::env::current_exe().expect("cannot get the path of the current test binary");
        let mut child = Command::new(&exe)
            .args(["--exact", "shutdown::tests::watchdog_child", "--nocapture"])
            .env(WATCHDOG_CHILD_ENV, "1")
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .expect("cannot spawn the watchdog child process");

        let status = tokio::time::timeout(Duration::from_secs(5), child.wait())
            .await
            .expect("the watchdog child process did not exit within 5 seconds")
            .expect("cannot wait for the watchdog child process");

        assert!(
            status.success(),
            "the watchdog child process exited with {status:?}"
        );
    }

    #[tokio::test]
    async fn watchdog_child() {
        if std::env::var(WATCHDOG_CHILD_ENV).is_err() {
            return;
        }
        spawn_watchdog(Duration::from_millis(150));
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
        }
    }

    #[cfg(unix)]
    mod unix_signals {
        use super::*;
        use nix::sys::signal::{Signal, kill};
        use nix::unistd::Pid;
        use std::future::Future;
        use std::io::Write;
        use tokio::io::{AsyncBufReadExt, BufReader};

        const SIGNAL_CHILD_ENV: &str = "BF_SHUTDOWN_TEST_SIGNAL_CHILD";

        async fn run_signal_child(os_signal: Signal) {
            let exe =
                std::env::current_exe().expect("cannot get the path of the current test binary");
            let mut child = Command::new(&exe)
                .args([
                    "--exact",
                    "shutdown::tests::unix_signals::signal_child_worker",
                    "--nocapture",
                ])
                .env(SIGNAL_CHILD_ENV, "1")
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .kill_on_drop(true)
                .spawn()
                .expect("cannot spawn the signal child process");

            let mut lines = BufReader::new(
                child
                    .stdout
                    .take()
                    .expect("the child process has no stdout pipe"),
            )
            .lines();

            let wait_for_ready = async {
                loop {
                    match lines.next_line().await {
                        Ok(Some(line)) if line == "READY" => return,
                        Ok(Some(_)) => continue,
                        Ok(None) => panic!("the child process exited before it wrote READY"),
                        Err(err) => panic!("cannot read stdout of the child process: {err}"),
                    }
                }
            };
            tokio::time::timeout(Duration::from_secs(5), wait_for_ready)
                .await
                .expect("the child process did not write READY within 5 seconds");

            let pid = Pid::from_raw(child.id().expect("the child process has no PID") as i32);
            kill(pid, os_signal).expect("cannot send the signal to the child process");

            let status = tokio::time::timeout(Duration::from_secs(5), child.wait())
                .await
                .expect("the child process did not exit within 5 seconds after the signal")
                .expect("cannot wait for the signal child process");
            assert!(status.success(), "the child process exited with {status:?}");
        }

        #[tokio::test]
        async fn sigterm_triggers_shutdown() {
            run_signal_child(Signal::SIGTERM).await;
        }

        #[tokio::test]
        async fn sigint_triggers_shutdown() {
            run_signal_child(Signal::SIGINT).await;
        }

        #[tokio::test]
        async fn signal_child_worker() {
            if std::env::var(SIGNAL_CHILD_ENV).is_err() {
                return;
            }

            let mut fut = Box::pin(signal());
            let waker = std::task::Waker::noop();
            let mut cx = std::task::Context::from_waker(waker);
            assert!(
                fut.as_mut().poll(&mut cx).is_pending(),
                "signal() resolved before the test sent a signal"
            );

            println!("\nREADY");
            std::io::stdout()
                .flush()
                .expect("cannot write READY to stdout");

            fut.await;
        }
    }
}
