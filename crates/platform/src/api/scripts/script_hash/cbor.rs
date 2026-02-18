use crate::{BlockfrostError, api::ApiResult};
use bf_api_provider::types::ScriptsCborResponse;

pub async fn route() -> ApiResult<ScriptsCborResponse> {
    Err(BlockfrostError::not_found())
}
