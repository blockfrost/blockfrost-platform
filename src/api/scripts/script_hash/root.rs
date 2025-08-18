use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::ScriptsSingleResponse;

pub async fn route() -> ApiResult<ScriptsSingleResponse> {
    Err(BlockfrostError::not_found())
}
