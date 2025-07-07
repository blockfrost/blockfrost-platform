use bf_common::{chain_config::ChainConfigCache, errors::AppError};
use pallas_network::miniprotocols::localstate::queries_v16::{CurrentProtocolParam, GenesisConfig};

use crate::pool::NodePool;

pub async fn init_caches(node_pool: NodePool) -> Result<ChainConfigCache, AppError> {
    let (genesis_config, protocol_params) = init_genesis_config(node_pool).await?;

    Ok(ChainConfigCache::new(genesis_config, protocol_params))
}

async fn init_genesis_config(
    node_pool: NodePool,
) -> Result<(GenesisConfig, CurrentProtocolParam), AppError> {
    let mut node = node_pool.get().await?;
    match node.genesis_config_and_pp().await {
        Ok((genesis_config, protocol_params)) => Ok((genesis_config, protocol_params)),
        Err(e) => Err(AppError::Server(format!(
            "Could not fetching genesis and protocol parameters. Is the Cardano node running? {e}"
        ))),
    }
}
