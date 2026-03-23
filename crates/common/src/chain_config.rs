use crate::helpers::system_start_to_epoch_millis;
use pallas_network::miniprotocols::localstate::queries_v16::{CurrentProtocolParam, GenesisConfig};
use serde::Serialize;

/// Cached chain configuration queried from the Cardano node at startup and
/// refreshed at epoch boundaries by [`ChainConfigWatch`].
pub struct ChainConfigCache {
    /// Shelley genesis configuration (network magic, system start, epoch length, etc.).
    pub genesis_config: GenesisConfig,
    /// Current protocol parameters (fees, execution unit prices, cost models, etc.).
    /// May change at epoch boundaries via governance actions.
    pub protocol_params: CurrentProtocolParam,
    /// Slot timing derived from genesis.
    pub slot_config: SlotConfig,
    /// Current Cardano era index (see [`Self::CONWAY_ERA`]).
    pub era: u16,
}

impl ChainConfigCache {
    /// Conway era index in the Cardano era sequence used by Ouroboros:
    /// Byron=0, Shelley=1, Allegra=2, Mary=3, Alonzo=4, Babbage=5, Conway=6.
    ///
    /// This is an application policy constant, not a runtime-discoverable value.
    /// The actual era is queried from the node via `get_current_era()` — this
    /// constant defines the minimum era we require. A future era bump will
    /// require code changes well beyond this constant (CBOR codecs, protocol
    /// params, pallas, testgen-hs), so hardcoding is intentional.
    pub const CONWAY_ERA: u16 = 6;

    pub fn new(
        genesis_config: GenesisConfig,
        protocol_params: CurrentProtocolParam,
    ) -> Result<Self, String> {
        let slot_config = SlotConfig::by_network_magic(&genesis_config);

        Ok(Self {
            genesis_config,
            protocol_params,
            slot_config,
            era: Self::CONWAY_ERA,
        })
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SlotConfig {
    /// Duration of a single slot in milliseconds (e.g. 1000 for all known networks).
    pub slot_length: u64,
    /// Absolute slot number at the start of the Shelley era (Byron/Shelley transition point).
    pub zero_slot: u64,
    /// Unix timestamp in milliseconds corresponding to `zero_slot`.
    pub zero_time: u64,
    /// Number of slots per epoch.
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
