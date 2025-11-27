use crate::{BlockfrostError, api::ApiResult};
use bf_api_provider::types::ScriptsSingleResponse;

pub async fn route() -> ApiResult<ScriptsSingleResponse> {
    Err(BlockfrostError::not_found())
}
