use crate::errors::APIError;
use blockfrost::{BlockFrostSettings, BlockfrostAPI as bf_sdk};

#[derive(Clone)]
pub struct BlockfrostAPI {
    api: bf_sdk,
}

impl BlockfrostAPI {
    pub fn new(project_id: &str) -> Self {
        let api = bf_sdk::new(project_id, BlockFrostSettings::default());

        BlockfrostAPI { api }
    }

    pub async fn nft_exists(&self, address: &str, asset: &str) -> Result<bool, APIError> {
        let bf_result = self
            .api
            .addresses(address)
            .await
            .map_err(|e| APIError::Unexpected(e.to_string()))?;

        let asset_exists = bf_result
            .amount
            .iter()
            .any(|x| x.unit == asset && x.quantity.parse::<i64>().unwrap_or(0) > 0);

        if asset_exists {
            Ok(true)
        } else {
            Err(APIError::License(address.to_string()))
        }
    }
}
