use crate::{
    errors::BlockfrostError,
    genesis::{GenesisRegistry, genesis},
    types::Network,
};
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
    pub fn from_path(epoch_number: String, network: &Network) -> Result<Self, BlockfrostError> {
        let network_data = genesis().by_network(network);
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
}
