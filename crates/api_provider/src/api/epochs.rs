use crate::types::EpochParamResponse;
use async_trait::async_trait;
use common::{errors::BlockfrostError, types::ApiResult};

#[async_trait]
pub trait EpochsApi: Send + Sync + 'static {
    async fn epoch_number_parameters(&self, _number: &str) -> ApiResult<EpochParamResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn epoch_latest_parameters(&self) -> ApiResult<EpochParamResponse> {
        Err(BlockfrostError::not_found())
    }
}
