use api_provider::types::BlocksSingleResponse;

use crate::{BlockfrostError, api::ApiResult};

pub async fn route() -> ApiResult<BlocksSingleResponse> {
    Err(BlockfrostError::not_found())
}
