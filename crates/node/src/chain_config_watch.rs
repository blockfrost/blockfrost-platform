use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use bf_common::chain_config::ChainConfigCache;
use bf_common::errors::BlockfrostError;
use tokio::sync::watch;
use tokio::time;

use crate::chain_config::init_caches;
use crate::pool::NodePool;

const RETRY_INTERVAL: Duration = Duration::from_secs(60);
/// Buffer after computed epoch boundary before re-querying protocol params
const EPOCH_BOUNDARY_BUFFER: Duration = Duration::from_secs(30);

/// Watches the Cardano node for sync readiness and epoch-boundary protocol
/// parameter changes, publishing [`ChainConfigCache`] updates via a
/// `tokio::sync::watch` channel.
#[derive(Clone)]
pub struct ChainConfigWatch {
    rx: watch::Receiver<Option<Arc<ChainConfigCache>>>,
}

impl ChainConfigWatch {
    /// Spawn the background monitor and return immediately
    ///
    /// The watch value starts as `None` and is set to `Some(…)` once the node
    /// is synced (tip within one epoch of expected slot)
    pub fn spawn(node_pool: NodePool) -> Self {
        let (tx, rx) = watch::channel(None);

        tokio::spawn(async move {
            monitor_loop(node_pool, tx).await;
        });

        Self { rx }
    }

    /// Returns the current config or a 503 if the node is not yet synced
    pub fn get(&self) -> Result<Arc<ChainConfigCache>, BlockfrostError> {
        self.rx.borrow().clone().ok_or_else(|| {
            BlockfrostError::service_unavailable(
                "Chain configuration is not yet available. The Cardano node may still be syncing."
                    .to_string(),
            )
        })
    }

    /// Wait until the first config is available (node synced and init complete)
    pub async fn wait_ready(&mut self) {
        while self.rx.borrow().is_none() {
            match self.rx.changed().await {
                Ok(_) => {},
                Err(err) => {
                    tracing::error!(
                        "ChainConfigWatch: watch channel closed before configuration became available: {err}"
                    );
                    break;
                },
            }
        }
    }
}

/// The main background loop: init config, then watch for epoch changes.
async fn monitor_loop(node_pool: NodePool, tx: watch::Sender<Option<Arc<ChainConfigCache>>>) {
    // Phase 1: first successful init, retried until chain tip is in the latest epoch
    let config = match init_until_success(&node_pool, &tx).await {
        Some(c) => c,
        None => return,
    };
    let mut slot_config = config.slot_config.clone();
    if tx.send(Some(Arc::new(config))).is_err() {
        return;
    }
    tracing::info!("ChainConfigWatch: chain configuration loaded");

    // Phase 2: watch for epoch changes.
    loop {
        let sleep_dur = duration_until_next_epoch(&slot_config);
        tokio::select! {
            () = time::sleep(sleep_dur) => {},
            () = tx.closed() => {
                tracing::info!("ChainConfigWatch: stopping monitor");
                return;
            },
        }

        let new_config = loop {
            match init_caches(node_pool.clone()).await {
                Ok(c) => break c,
                Err(e) => {
                    tracing::warn!(
                        "ChainConfigWatch: failed to refresh chain config: {e}. Retrying in {}s.",
                        RETRY_INTERVAL.as_secs()
                    );
                    tokio::select! {
                        () = time::sleep(RETRY_INTERVAL) => {},
                        () = tx.closed() => {
                            tracing::info!("ChainConfigWatch: stopping monitor");
                            return;
                        },
                    }
                },
            }
        };

        slot_config = new_config.slot_config.clone();
        if tx.send(Some(Arc::new(new_config))).is_err() {
            break;
        }
    }
}

/// Retry `init_caches` until it succeeds
/// Returns `None` if all receivers were dropped (shutdown)
async fn init_until_success(
    node_pool: &NodePool,
    tx: &watch::Sender<Option<Arc<ChainConfigCache>>>,
) -> Option<ChainConfigCache> {
    loop {
        match init_caches(node_pool.clone()).await {
            Ok(config) => return Some(config),
            Err(e) => {
                tracing::debug!(
                    "ChainConfigWatch: failed to load chain config from node: {e}. Retrying in {}s.",
                    RETRY_INTERVAL.as_secs()
                );
                tokio::select! {
                    () = time::sleep(RETRY_INTERVAL) => {},
                    () = tx.closed() => return None,
                }
            },
        }
    }
}

/// Compute how long to sleep until the next epoch boundary (plus buffer).
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
        assert!(result >= EPOCH_BOUNDARY_BUFFER);
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
    fn test_exact_epoch_boundary_waits_full_epoch() {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis() as u64;
        let config = SlotConfig {
            slot_length: 1000,
            zero_slot: 0,
            zero_time: now_ms, // current time → current_slot = 0 → slot_in_epoch = 0
            epoch_length: 100,
        };
        let result = duration_until_next_epoch(&config);
        let full_epoch = Duration::from_millis(config.epoch_length * config.slot_length);
        // Should be approximately full_epoch + buffer, not just buffer
        assert!(
            result > full_epoch,
            "At epoch boundary, expected > {full_epoch:?}, got {result:?}"
        );
    }

    #[test]
    fn test_mainnet_config() {
        let config = SlotConfig::mainnet();
        let max =
            Duration::from_millis(config.epoch_length * config.slot_length) + EPOCH_BOUNDARY_BUFFER;
        let result = duration_until_next_epoch(&config);
        assert!(result >= EPOCH_BOUNDARY_BUFFER);
        assert!(result <= max);
    }
}
