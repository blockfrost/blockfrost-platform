use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::PoolsSingleResponse;

pub async fn route() -> ApiResult<PoolsSingleResponse> {
    Err(BlockfrostError::not_found())
}
