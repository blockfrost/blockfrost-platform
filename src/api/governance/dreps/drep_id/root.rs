use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::DrepSingleResponse;

pub async fn route() -> ApiResult<DrepSingleResponse> {
    Err(BlockfrostError::not_found())
}
