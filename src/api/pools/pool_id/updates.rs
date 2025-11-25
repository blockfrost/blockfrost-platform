use crate::{BlockfrostError, api::ApiResult};
use bf_api_provider::types::PoolsUpdatesResponse;

pub async fn route() -> ApiResult<PoolsUpdatesResponse> {
    Err(BlockfrostError::not_found())
}
