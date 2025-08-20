use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::DrepsUpdatesResponse;

pub async fn route() -> ApiResult<DrepsUpdatesResponse> {
    Err(BlockfrostError::not_found())
}
