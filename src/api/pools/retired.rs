use crate::{BlockfrostError, api::ApiResult};
use bf_api_provider::types::PoolsRetiresResponse;

pub async fn route() -> ApiResult<PoolsRetiresResponse> {
    Err(BlockfrostError::not_found())
}
