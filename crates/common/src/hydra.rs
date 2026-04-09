#[cfg(unix)]
use std::time::Duration;
#[cfg(unix)]
use tracing::warn;

/// A validated machine identifier, a 64-character hex string (BLAKE3 digest).
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(try_from = "String", into = "String")]
pub struct MachineId(String);

/// Error returned when a [`MachineId`] fails validation.
#[derive(Debug)]
pub struct InvalidMachineId {
    len: usize,
}

impl std::fmt::Display for InvalidMachineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "invalid machine_id (len={}): expected exactly 64 hex characters",
            self.len,
        )
    }
}

impl std::error::Error for InvalidMachineId {}

impl TryFrom<String> for MachineId {
    type Error = InvalidMachineId;

    fn try_from(s: String) -> Result<Self, Self::Error> {
        if s.len() == 64 && s.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            Ok(Self(s))
        } else {
            Err(InvalidMachineId { len: s.len() })
        }
    }
}

impl From<MachineId> for String {
    fn from(m: MachineId) -> Self {
        m.0
    }
}

impl std::fmt::Display for MachineId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl AsRef<str> for MachineId {
    fn as_ref(&self) -> &str {
        &self.0
    }
}

impl MachineId {
    /// BLAKE3 hash of the current host's machine-uid.
    ///
    /// Used to detect whether two communicating processes (e.g. Gateway and
    /// Bridge) are co-located on the same host, in which case TCP-over-WebSocket
    /// tunnelling must be skipped, because the respective ports are already taken.
    pub fn of_this_host() -> Self {
        const MACHINE_ID_NAMESPACE: &str = "blockfrost.machine-id.v1";

        let mut hasher = blake3::Hasher::new();
        hasher.update(MACHINE_ID_NAMESPACE.as_bytes());
        hasher.update(b":");

        match machine_uid::get() {
            Ok(id) => {
                hasher.update(id.as_bytes());
            },
            Err(e) => {
                tracing::warn!(error = ?e, "machine_uid::get() failed; falling back to random bytes");
                let mut fallback = [0u8; 32];
                getrandom::fill(&mut fallback)
                    .expect("getrandom::fill shouldn't fail in normal circumstances");
                hasher.update(&fallback);
            },
        }

        hasher
            .finalize()
            .to_hex()
            .to_string()
            .try_into()
            .expect("blake3 hex output is always a valid MachineId")
    }
}

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
