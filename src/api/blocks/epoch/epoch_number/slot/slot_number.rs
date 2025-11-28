use crate::{BlockfrostError, api::ApiResult};
use bf_api_provider::types::BlocksResponse;

pub async fn route() -> ApiResult<BlocksResponse> {
    Err(BlockfrostError::not_found())
}
