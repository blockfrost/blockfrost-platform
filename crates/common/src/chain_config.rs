use crate::helpers::system_start_to_epoch_millis;
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
    pub fn new(
        genesis_config: GenesisConfig,
        protocol_params: CurrentProtocolParam,
    ) -> Result<Self, String> {
        let slot_config = SlotConfig::by_network_magic(&genesis_config);

        Ok(Self {
            genesis_config,
            protocol_params,
            slot_config,
            // The era number in the Cardano era sequence used by Ogmios/testgen-hs:
            // Byron=0, Shelley=1, Allegra=2, Mary=3, Alonzo=4, Babbage=5, Conway=6.
            // Hardcoded to Conway (6) since that is the only era we currently support.
            era: 6,
        })
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

    pub fn by_network_magic(genesis_config: &GenesisConfig) -> Self {
        match genesis_config.network_magic {
            764824073 => Self::mainnet(),
            1 => Self::preprod(),
            2 => Self::preview(),
            _ => Self::from_genesis_config(genesis_config),
        }
    }

    /// Derive slot config from genesis for custom/unknown networks.
    /// Assumes no Byron era (zero_slot = 0, zero_time = system_start).
    fn from_genesis_config(genesis_config: &GenesisConfig) -> Self {
        Self {
            slot_length: genesis_config.slot_length as u64 * 1000,
            zero_slot: 0,
            zero_time: system_start_to_epoch_millis(&genesis_config.system_start),
            epoch_length: genesis_config.epoch_length as u64,
        }
    }
}
