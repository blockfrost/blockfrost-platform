use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::PoolsRetiresResponse;

pub async fn route() -> ApiResult<PoolsRetiresResponse> {
    Err(BlockfrostError::not_found())
}
