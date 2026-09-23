use std::time::Duration;

use tracing::{info, warn};

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
/// [`std::process::exit`], so `atexit` handlers still run.
pub async fn signal() {
    #[cfg(unix)]
    {
        use tokio::signal::unix::{SignalKind, signal};

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

    // A plain thread, so that it survives the Tokio runtime being dropped:
    std::thread::spawn(|| {
        std::thread::sleep(GRACE_PERIOD);
        eprintln!(
            "Graceful shutdown didn’t finish within {}s, exiting",
            GRACE_PERIOD.as_secs()
        );
        std::process::exit(1);
    });
}
