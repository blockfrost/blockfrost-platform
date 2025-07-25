use crate::types::{NetworkErasResponse, NetworkResponse};
use async_trait::async_trait;
use common::{errors::BlockfrostError, types::ApiResult};

#[async_trait]
pub trait NetworkApi: Send + Sync + 'static {
    async fn network(&self) -> ApiResult<NetworkResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn network_eras(&self) -> ApiResult<NetworkErasResponse> {
        Err(BlockfrostError::not_found())
    }
}
