use crate::{BlockfrostError, api::ApiResult};
use bf_api_provider::types::PoolsSingleResponse;

pub async fn route() -> ApiResult<PoolsSingleResponse> {
    Err(BlockfrostError::not_found())
}
