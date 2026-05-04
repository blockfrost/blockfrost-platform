use bf_common::{
    chain_config::{ChainConfigCache, SlotConfig},
    errors::AppError,
};

use crate::pool::NodePool;

/// inits the cache only if the chain tip is in the latest epoch
pub async fn init_caches(node_pool: NodePool) -> Result<ChainConfigCache, AppError> {
    let mut node = node_pool.get().await?;
    let (genesis_config, protocol_params, tip_slot) =
        node.genesis_config_and_pp().await.map_err(|e| {
            AppError::Server(format!(
                "Could not fetch genesis and protocol parameters. Is the Cardano node running? {e}"
            ))
        })?;

    let config =
        ChainConfigCache::new(genesis_config, protocol_params).map_err(AppError::Server)?;

    // Node's tip must be within the last epoch
    let expected_slot = current_slot(&config.slot_config);
    if expected_slot.saturating_sub(tip_slot) > config.slot_config.epoch_length {
        return Err(AppError::Server(format!(
            "Node tip slot ({tip_slot}) is more than one epoch behind expected slot ({expected_slot})"
        )));
    }

    Ok(config)
}

fn current_slot(slot_config: &SlotConfig) -> u64 {
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let elapsed_ms = now_ms.saturating_sub(slot_config.zero_time);
    slot_config.zero_slot + elapsed_ms / slot_config.slot_length
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_slot_future_zero_time_returns_zero_slot() {
        let config = SlotConfig {
            slot_length: 1000,
            zero_slot: 100,
            zero_time: u64::MAX,
            epoch_length: 432000,
        };
        assert_eq!(current_slot(&config), 100);
    }
}
