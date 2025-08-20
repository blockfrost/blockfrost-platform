use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::PoolsUpdatesResponse;

pub async fn route() -> ApiResult<PoolsUpdatesResponse> {
    Err(BlockfrostError::not_found())
}
