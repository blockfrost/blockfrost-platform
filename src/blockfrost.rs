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
            .map_err(|_| APIError::UnexpectedError())?;

        let asset_exists = bf_result.amount.iter().any(|x| {
            x.unit == asset && {
                match x.quantity.parse::<i64>() {
                    Ok(quantity) => quantity > 0,
                    Err(_) => false,
                }
            }
        });

        if asset_exists {
            Ok(true)
        } else {
            Err(APIError::LicenseError(address.to_string()))
        }
    }
}
