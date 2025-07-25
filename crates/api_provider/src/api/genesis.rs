use crate::types::GenesisResponse;
use async_trait::async_trait;
use common::{errors::BlockfrostError, types::ApiResult};

#[async_trait]
pub trait GenesisApi: Send + Sync + 'static {
    async fn genesis(&self) -> ApiResult<GenesisResponse> {
        Err(BlockfrostError::not_found())
    }
}
