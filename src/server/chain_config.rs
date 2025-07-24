use pallas_network::miniprotocols::localstate::queries_v16::{CurrentProtocolParam, GenesisConfig};
use serde::Serialize;

use crate::{AppError, NodePool};

/// This structure is used to share server-wide cachables
pub struct ChainConfigCache {
    pub genesis_config: GenesisConfig,
    pub protocol_params: CurrentProtocolParam,
    pub slot_config: SlotConfig,
    pub era: u16,
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotConfig {
    pub slot_length: u64,
    pub zero_slot: u64,
    pub zero_time: u64,
    pub epoch_length: u64,
}

impl Default for SlotConfig {
    fn default() -> Self {
        Self {
            slot_length: 1000,
            zero_slot: 4492800,
            zero_time: 1596059091000,
            epoch_length: 432000,
        }
    }
}

impl ChainConfigCache {
    /// init various caches
    pub async fn init_caches(node_pool: NodePool) -> Result<Self, AppError> {
        let (genesis_config, protocol_params) = Self::init_genesis_config(node_pool).await?;

        Ok(Self {
            genesis_config,
            protocol_params,
            slot_config: SlotConfig::default(),
            era: 6, //conway
        })
    }

    async fn init_genesis_config(
        node_pool: NodePool,
    ) -> Result<(GenesisConfig, CurrentProtocolParam), AppError> {
        let mut node: deadpool::managed::Object<crate::node::pool_manager::NodePoolManager> =
            node_pool.get().await?;
        match node.genesis_config_and_pp().await {
            Ok((genesis_config, protocol_params)) => Ok((genesis_config, protocol_params)),
            Err(e) => Err(AppError::Server(format!(
                "Could not fetching genesis and protocol parameters. Is the Cardano node running? {e}"
            ))),
        }
    }
}
