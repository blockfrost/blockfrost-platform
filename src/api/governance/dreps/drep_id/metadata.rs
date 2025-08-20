use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::DrepsMetadataResponse;

pub async fn route() -> ApiResult<DrepsMetadataResponse> {
    Err(BlockfrostError::not_found())
}
