use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::DrepsSingleResponse;

pub async fn route() -> ApiResult<DrepsSingleResponse> {
    Err(BlockfrostError::not_found())
}
