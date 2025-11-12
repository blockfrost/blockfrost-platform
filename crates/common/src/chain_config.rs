use pallas_network::miniprotocols::localstate::queries_v16::{CurrentProtocolParam, GenesisConfig};
use serde::Serialize;

/// This structure is used to share server-wide cachables
pub struct ChainConfigCache {
    pub genesis_config: GenesisConfig,
    pub protocol_params: CurrentProtocolParam,
    pub slot_config: SlotConfig,
    pub era: u16,
}

impl ChainConfigCache {
    pub fn new(genesis_config: GenesisConfig, protocol_params: CurrentProtocolParam) -> Self {
        let slot_config = SlotConfig::by_network_magic(&genesis_config.network_magic);

        Self {
            genesis_config,
            protocol_params,
            slot_config,
            era: 6, // conway
        }
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotConfig {
    pub slot_length: u64,
    pub zero_slot: u64,
    pub zero_time: u64,
    pub epoch_length: u64,
}

impl SlotConfig {
    pub fn mainnet() -> Self {
        Self {
            slot_length: 1000,
            zero_slot: 4492800,
            zero_time: 1596059091000,
            epoch_length: 432000,
        }
    }

    pub fn preprod() -> Self {
        Self {
            slot_length: 1000,
            zero_slot: 86400,
            zero_time: 1655683200000,
            epoch_length: 432000,
        }
    }

    pub fn preview() -> Self {
        Self {
            slot_length: 1000,
            zero_slot: 0,
            zero_time: 1666656000000,
            epoch_length: 86400,
        }
    }

    pub fn by_network_magic(network_magic: &u32) -> Self {
        match network_magic {
            764824073 => Self::preview(),
            1 => Self::mainnet(),
            2 => Self::preprod(),
            _ => Self::default(),
        }
    }
}

impl Default for SlotConfig {
    fn default() -> Self {
        SlotConfig::mainnet()
    }
}
