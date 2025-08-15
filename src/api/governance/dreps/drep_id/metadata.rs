use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::DrepMetadataResponse;

pub async fn route() -> ApiResult<DrepMetadataResponse> {
    Err(BlockfrostError::not_found())
}
