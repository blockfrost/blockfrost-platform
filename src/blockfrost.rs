use crate::errors::APIError;
use blockfrost::{BlockFrostSettings, BlockfrostAPI as bf_sdk};
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct BlockfrostAPI {
    api: bf_sdk,
    policy_id_size: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AssetName(pub String);
impl AssetName {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

pub struct Asset {
    pub asset_name: AssetName,
}

impl BlockfrostAPI {
    pub fn new(project_id: &str) -> Self {
        let api = bf_sdk::new(project_id, BlockFrostSettings::default());

        BlockfrostAPI {
            api,
            policy_id_size: 56,
        }
    }

    // Parse asset from the unit
    async fn parse_asset(&self, unit: &str) -> Result<Asset, APIError> {
        if unit.len() < self.policy_id_size {
            return Err(APIError::License(
                "Unit is too short to contain a valid policy ID".to_string(),
            ));
        }

        let asset_hex = &unit[self.policy_id_size..];
        let decoded = hex::decode(asset_hex)
            .map_err(|err| APIError::License(format!("Hex decoding failed: {}", err)))?;

        let asset_name = AssetName(String::from_utf8_lossy(&decoded).to_string());

        Ok(Asset { asset_name })
    }

    // Check if NFT exists at the address
    pub async fn nft_exists(&self, address: &str, asset: &str) -> Result<Asset, APIError> {
        if cfg!(feature = "dev_mock_db") {
            return Ok(Asset {
                asset_name: AssetName("IcebreakerX".to_string()),
            });
        }

        let bf_result = self
            .api
            .addresses(address)
            .await
            .map_err(|err| APIError::License(err.to_string()))?;

        let found_asset = bf_result
            .amount
            .iter()
            .filter(|x| x.unit != "lovelace")
            .find(|x| {
                x.unit.len() >= self.policy_id_size
                    && &x.unit[..self.policy_id_size] == asset
                    && x.quantity.parse::<i64>().unwrap_or(0) > 0
            });

        let found_asset_unit = match found_asset {
            Some(a) => &a.unit,
            None => return Err(APIError::License("Asset not found".to_string())),
        };

        self.parse_asset(found_asset_unit).await
    }
}
