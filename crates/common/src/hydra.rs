#[cfg(unix)]
use std::time::Duration;
#[cfg(unix)]
use tracing::warn;

/// Send `SIGTERM` to every process in the group identified by `pgid`,
/// poll until all members have exited, and escalate to `SIGKILL` after 5 s.
/// Gives up after 10 s so we never block the caller forever.
///
/// `hydra-node` is spawned with `setpgid(0, 0)`, so its PID equals its PGID
/// and all its descendants (e.g. `etcd`) share the same group.
#[cfg(unix)]
pub async fn kill_and_wait_process_group(pgid: u32) {
    let neg = -(pgid as i32);

    // SIGTERM the entire group (hydra-node + children like etcd).
    // Safety: we created this process group via setpgid(0,0) at spawn.
    unsafe {
        nix::libc::kill(neg, nix::libc::SIGTERM);
    }

    let start = tokio::time::Instant::now();
    let mut escalated = false;

    loop {
        // kill(pid, 0) checks whether any group member is still alive
        // without actually sending a signal.
        if unsafe { nix::libc::kill(neg, 0) } == -1 {
            // ESRCH (or any other error) → no reachable processes remain.
            return;
        }

        let elapsed = start.elapsed();

        if !escalated && elapsed >= Duration::from_secs(5) {
            warn!(
                "hydra-node process group {pgid} still alive after {elapsed:?}, \
                 escalating to SIGKILL"
            );
            unsafe {
                nix::libc::kill(neg, nix::libc::SIGKILL);
            }
            escalated = true;
        }

        // Hard safety limit so we never block the caller forever.
        if elapsed >= Duration::from_secs(10) {
            warn!(
                "hydra-node process group {pgid} still present after {elapsed:?}, \
                 proceeding anyway"
            );
            return;
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}
