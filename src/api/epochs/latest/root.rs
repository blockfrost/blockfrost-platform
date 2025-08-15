use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::EpochsResponse;

pub async fn route() -> ApiResult<EpochsResponse> {
    Err(BlockfrostError::not_found())
}
