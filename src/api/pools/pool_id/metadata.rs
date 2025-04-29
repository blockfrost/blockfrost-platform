use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::pool_metadata::PoolMetadata;

pub async fn route() -> ApiResult<PoolMetadata> {
    Err(BlockfrostError::not_found())
}
