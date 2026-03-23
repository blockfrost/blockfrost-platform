use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bf_common::chain_config::ChainConfigCache;
use bf_common::errors::BlockfrostError;
use tokio::sync::watch;
use tokio::time;

use crate::chain_config::init_caches;
use crate::pool::NodePool;

const CONWAY_ERA: u16 = ChainConfigCache::CONWAY_ERA;
const SYNC_THRESHOLD: f64 = 99.9;
const SYNC_POLL_INTERVAL: Duration = Duration::from_secs(60);
const RETRY_INTERVAL: Duration = Duration::from_secs(60);
/// Buffer after computed epoch boundary before re-querying protocol params.
const EPOCH_BOUNDARY_BUFFER: Duration = Duration::from_secs(30);
/// Error substring returned by `sync_progress()` for non-well-known networks.
const UNSUPPORTED_NETWORK_ERROR: &str = "Only well-known networks";

/// Watches the Cardano node for sync readiness and epoch-boundary protocol
/// parameter changes, publishing [`ChainConfigCache`] updates via a
/// `tokio::sync::watch` channel.
#[derive(Clone)]
pub struct ChainConfigWatch {
    rx: watch::Receiver<Option<Arc<ChainConfigCache>>>,
}

impl ChainConfigWatch {
    /// Spawn the background monitor and return immediately.
    ///
    /// The watch value starts as `None` and is set to `Some(…)` once the node
    /// reaches the Conway era with sufficient sync progress.
    pub fn spawn(node_pool: NodePool) -> Self {
        let (tx, rx) = watch::channel(None);

        tokio::spawn(async move {
            monitor_loop(node_pool, tx).await;
        });

        Self { rx }
    }

    /// Returns the current config or a 503 if the node is not yet synced.
    pub fn get(&self) -> Result<Arc<ChainConfigCache>, BlockfrostError> {
        self.rx.borrow().clone().ok_or_else(|| {
            BlockfrostError::service_unavailable(
                "Chain configuration is not yet available. The Cardano node may still be syncing."
                    .to_string(),
            )
        })
    }

    /// Wait until the first config is available (node synced and init complete).
    pub async fn wait_ready(&mut self) {
        while self.rx.borrow().is_none() {
            if self.rx.changed().await.is_err() {
                break;
            }
        }
    }
}

/// The main background loop: wait for sync, init, then watch for epoch changes.
async fn monitor_loop(node_pool: NodePool, tx: watch::Sender<Option<Arc<ChainConfigCache>>>) {
    // Phase 1 – wait until the node is synced (or ready enough).
    wait_for_sync(&node_pool).await;

    // Phase 2 – first successful init.
    let config = init_until_success(&node_pool).await;
    let slot_config = config.slot_config.clone();
    let _ = tx.send(Some(Arc::new(config)));
    tracing::info!("ChainConfigWatch: chain configuration loaded successfully");

    // Phase 3 – watch for epoch changes.
    loop {
        let sleep_dur = duration_until_next_epoch(&slot_config);
        tracing::info!(
            sleep_secs = sleep_dur.as_secs(),
            "ChainConfigWatch: next epoch boundary check scheduled"
        );
        time::sleep(sleep_dur).await;

        let new_config = loop {
            match init_caches(node_pool.clone()).await {
                Ok(c) => break c,
                Err(e) => {
                    tracing::error!(
                        "ChainConfigWatch: failed to refresh chain config at epoch boundary: {e}. \
                         Current config remains active, retrying in {}s. \
                         If this persists, check your Cardano node connectivity.",
                        RETRY_INTERVAL.as_secs()
                    );
                    time::sleep(RETRY_INTERVAL).await;
                },
            }
        };

        let params_changed = tx
            .borrow()
            .as_ref()
            .is_none_or(|old| old.protocol_params != new_config.protocol_params);

        if params_changed {
            tracing::info!(
                "ChainConfigWatch: protocol parameters changed at epoch boundary — reloading chain config"
            );
            let _ = tx.send(Some(Arc::new(new_config)));
        } else {
            tracing::debug!(
                "ChainConfigWatch: epoch boundary reached, protocol parameters unchanged — no action needed"
            );
        }
    }
}

/// Block until the node reports Conway era and sync progress ≥ threshold.
///
/// For custom (non-well-known) networks, `sync_progress()` is not available, so
/// we skip the sync check and rely on `init_caches` succeeding.
async fn wait_for_sync(node_pool: &NodePool) {
    loop {
        match check_sync(node_pool).await {
            SyncStatus::Ready => return,
            SyncStatus::NotReady(reason) => {
                tracing::info!(
                    "ChainConfigWatch: {reason}, retrying in {}s",
                    SYNC_POLL_INTERVAL.as_secs()
                );
                time::sleep(SYNC_POLL_INTERVAL).await;
            },
            SyncStatus::CustomNetwork => {
                tracing::info!(
                    "ChainConfigWatch: custom network detected — skipping sync check, will attempt init directly"
                );
                return;
            },
        }
    }
}

enum SyncStatus {
    Ready,
    NotReady(String),
    CustomNetwork,
}

async fn check_sync(node_pool: &NodePool) -> SyncStatus {
    let mut node = match node_pool.get().await {
        Ok(n) => n,
        Err(e) => {
            return SyncStatus::NotReady(format!(
                "Cannot connect to Cardano node: {e}. \
                 Ensure node is running and socket path is correct"
            ));
        },
    };

    match node.sync_progress().await {
        Ok(info) => {
            if info.era != CONWAY_ERA {
                SyncStatus::NotReady(format!(
                    "Node is in era {} (need Conway era {CONWAY_ERA})",
                    info.era
                ))
            } else if info.sync_progress < SYNC_THRESHOLD {
                SyncStatus::NotReady(format!(
                    "Node sync progress {:.2}% < {SYNC_THRESHOLD}% threshold",
                    info.sync_progress
                ))
            } else {
                SyncStatus::Ready
            }
        },
        Err(e) => {
            let msg = e.to_string();
            if msg.contains(UNSUPPORTED_NETWORK_ERROR) {
                SyncStatus::CustomNetwork
            } else {
                SyncStatus::NotReady(format!(
                    "Cannot query node sync status: {e}. \
                     Ensure the Cardano node is running"
                ))
            }
        },
    }
}

/// Retry `init_caches` until it succeeds.
async fn init_until_success(node_pool: &NodePool) -> ChainConfigCache {
    loop {
        match init_caches(node_pool.clone()).await {
            Ok(config) => return config,
            Err(e) => {
                tracing::warn!(
                    "ChainConfigWatch: failed to load chain config from node: {e}. \
                     Retrying in {}s.",
                    RETRY_INTERVAL.as_secs()
                );
                time::sleep(RETRY_INTERVAL).await;
            },
        }
    }
}

/// Compute how long to sleep until the next epoch boundary (plus buffer).
///
/// Returns [`RETRY_INTERVAL`] as a fallback if `slot_config` contains zero
/// values that would cause a division-by-zero.
fn duration_until_next_epoch(slot_config: &bf_common::chain_config::SlotConfig) -> Duration {
    if slot_config.slot_length == 0 || slot_config.epoch_length == 0 {
        return RETRY_INTERVAL;
    }

    let now_ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;

    // Current slot (may be approximate but close enough for scheduling).
    let elapsed_ms = now_ms.saturating_sub(slot_config.zero_time);
    let current_slot = slot_config.zero_slot + elapsed_ms / slot_config.slot_length;

    // Slots into the current epoch.
    let slot_in_epoch = (current_slot - slot_config.zero_slot) % slot_config.epoch_length;
    let slots_until_next = slot_config.epoch_length - slot_in_epoch;

    let ms_until_next = slots_until_next * slot_config.slot_length;
    Duration::from_millis(ms_until_next) + EPOCH_BOUNDARY_BUFFER
}

#[cfg(test)]
mod tests {
    use super::*;
    use bf_common::chain_config::SlotConfig;

    #[test]
    fn test_zero_slot_length_returns_retry_interval() {
        let config = SlotConfig {
            slot_length: 0,
            zero_slot: 0,
            zero_time: 0,
            epoch_length: 432000,
        };
        assert_eq!(duration_until_next_epoch(&config), RETRY_INTERVAL);
    }

    #[test]
    fn test_zero_epoch_length_returns_retry_interval() {
        let config = SlotConfig {
            slot_length: 1000,
            zero_slot: 0,
            zero_time: 0,
            epoch_length: 0,
        };
        assert_eq!(duration_until_next_epoch(&config), RETRY_INTERVAL);
    }

    #[test]
    fn test_result_includes_buffer() {
        let result = duration_until_next_epoch(&SlotConfig::preview());
        assert!(result > EPOCH_BOUNDARY_BUFFER);
    }

    #[test]
    fn test_result_at_most_one_epoch() {
        let config = SlotConfig::preview();
        let max =
            Duration::from_millis(config.epoch_length * config.slot_length) + EPOCH_BOUNDARY_BUFFER;
        let result = duration_until_next_epoch(&config);
        assert!(result <= max, "result {result:?} exceeds max {max:?}");
    }

    #[test]
    fn test_mainnet_config() {
        let config = SlotConfig::mainnet();
        let max =
            Duration::from_millis(config.epoch_length * config.slot_length) + EPOCH_BOUNDARY_BUFFER;
        let result = duration_until_next_epoch(&config);
        assert!(result > EPOCH_BOUNDARY_BUFFER);
        assert!(result <= max);
    }
}
