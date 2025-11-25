use crate::{BlockfrostError, api::ApiResult};
use bf_api_provider::types::PoolsMetadataResponse;

pub async fn route() -> ApiResult<PoolsMetadataResponse> {
    Err(BlockfrostError::not_found())
}
