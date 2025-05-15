use crate::{addresses::is_stake_address_valid, config::Network, errors::BlockfrostError};

use serde::Deserialize;

#[derive(Deserialize)]
pub struct AccountsPath {
    pub stake_address: String,
}

pub struct AccountData {
    pub stake_address: String,
}

impl AccountData {
    pub fn from_account_path(
        stake_address: String,
        network: &Network,
    ) -> Result<Self, BlockfrostError> {
        let is_valid = is_stake_address_valid(&stake_address, network)?;

        if !is_valid {
            return Err(BlockfrostError::invalid_stake_address());
        }

        Ok(Self {
            stake_address: stake_address.to_string(),
        })
    }
}
