use bf_common::{chain_config::ChainConfigCache, errors::AppError};
use pallas_network::miniprotocols::localstate::queries_v16::{CurrentProtocolParam, GenesisConfig};

use crate::pool::NodePool;

pub async fn init_caches(node_pool: NodePool) -> Result<ChainConfigCache, AppError> {
    let (genesis_config, protocol_params) = init_genesis_config(node_pool).await?;

    ChainConfigCache::new(genesis_config, protocol_params).map_err(AppError::Server)
}

async fn init_genesis_config(
    node_pool: NodePool,
) -> Result<(GenesisConfig, CurrentProtocolParam), AppError> {
    let mut node = node_pool.get().await?;
    node.genesis_config_and_pp().await.map_err(|e| {
        AppError::Server(format!(
            "Could not fetch genesis and protocol parameters. Is the Cardano node running? {e}"
        ))
    })
}
