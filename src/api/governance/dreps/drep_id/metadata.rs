use crate::{BlockfrostError, api::ApiResult};
use bf_api_provider::types::DrepsMetadataResponse;

pub async fn route() -> ApiResult<DrepsMetadataResponse> {
    Err(BlockfrostError::not_found())
}
