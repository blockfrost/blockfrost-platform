use blockfrost::{BlockFrostSettings, BlockfrostAPI as bf_sdk};

use crate::errors::APIError;

pub struct BlockfrostAPI {
    api: bf_sdk,
}

impl BlockfrostAPI {
    pub fn new(project_id: &str) -> bf_sdk {
        bf_sdk::new(project_id, BlockFrostSettings::default())
    }

    pub async fn nft_exists(&self, address: &str, policy_id: &str) -> Result<bool, APIError> {
        Ok(true)
    }
}
