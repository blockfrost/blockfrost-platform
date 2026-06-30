use crate::genesis::GenesisRegistry;
use bf_api_provider::types::GenesisResponse;
use bf_common::{errors::BlockfrostError, types::Network};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct EpochsPath {
    pub epoch_number: String,
}

pub struct EpochData {
    pub epoch_number: i32,
    pub epoch_length: i32,
}

impl EpochData {
    pub fn from_path(
        epoch_number: String,
        network: &Network,
        genesis: &[(Network, GenesisResponse)],
    ) -> Result<Self, BlockfrostError> {
        let network_data = genesis.by_network(network);
        let epoch_length = network_data.epoch_length;

        if !epoch_number.chars().all(|c| c.is_ascii_digit()) {
            return Err(BlockfrostError::invalid_epoch_number());
        }

        if !Self::is_positive_int(Some(&epoch_number)) {
            return Err(BlockfrostError::invalid_epoch_missing_or_malformed());
        }

        match epoch_number.parse::<i32>() {
            Ok(epoch_number) => Ok(Self {
                epoch_number,
                epoch_length,
            }),
            Err(_) => Err(BlockfrostError::invalid_epoch_number()),
        }
    }

    pub fn is_positive_int(possible_positive_int: Option<&str>) -> bool {
        match possible_positive_int {
            Some(s) => match s.parse::<i32>() {
                Ok(val) => (0..=i32::MAX).contains(&val),
                Err(_) => false,
            },
            None => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::genesis::{GenesisRegistryMut, genesis};
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case("0", true)]
    #[case("21447", true)]
    #[case("2147483647", true)]
    #[case("-1", false)]
    #[case("2147483648", false)]
    #[case("NaN", false)]
    #[case("69696969", true)]
    #[case("", false)]
    fn test_is_positive_int(#[case] value: &str, #[case] expected: bool) {
        use crate::epochs::EpochData;

        assert_eq!(EpochData::is_positive_int(Some(value)), expected);
    }

    fn dummy_genesis(network_magic: i32, epoch_length: i32) -> GenesisResponse {
        GenesisResponse {
            active_slots_coefficient: 0.05,
            update_quorum: 5,
            max_lovelace_supply: "1".to_string(),
            network_magic,
            epoch_length,
            system_start: 0,
            slots_per_kes_period: 1,
            slot_length: 1,
            max_kes_evolutions: 1,
            security_param: 1,
        }
    }

    #[test]
    fn from_path_reads_epoch_length_from_builtin_network() {
        let registry = genesis();
        let epoch = EpochData::from_path("100".to_string(), &Network::Preview, &registry)
            .expect("valid epoch");
        assert_eq!(epoch.epoch_number, 100);
        // Preview's epoch_length from the built-in registry.
        assert_eq!(epoch.epoch_length, 86_400);
    }

    #[test]
    fn from_path_reads_epoch_length_from_custom_network() {
        let mut registry = genesis();
        registry.add(Network::Custom, dummy_genesis(42, 777));

        let epoch = EpochData::from_path("5".to_string(), &Network::Custom, &registry)
            .expect("valid epoch");
        assert_eq!(epoch.epoch_number, 5);
        assert_eq!(epoch.epoch_length, 777);
    }

    #[test]
    fn from_path_rejects_non_numeric_epoch() {
        let registry = genesis();
        let err = EpochData::from_path("abc".to_string(), &Network::Preview, &registry);
        assert!(err.is_err());
    }
}
