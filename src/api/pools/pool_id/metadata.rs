use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::PoolsMetadataResponse;

pub async fn route() -> ApiResult<PoolsMetadataResponse> {
    Err(BlockfrostError::not_found())
}
