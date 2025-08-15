use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::DrepUpdatesResponse;

pub async fn route() -> ApiResult<DrepUpdatesResponse> {
    Err(BlockfrostError::not_found())
}
