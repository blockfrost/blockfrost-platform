use bf_api_provider::types::DrepsMetadataResponse;

use crate::{BlockfrostError, api::ApiResult};

pub async fn route() -> ApiResult<DrepsMetadataResponse> {
    Err(BlockfrostError::not_found())
}
