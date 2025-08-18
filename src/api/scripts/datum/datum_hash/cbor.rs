use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::ScriptsDatumCborResponse;

pub async fn route() -> ApiResult<ScriptsDatumCborResponse> {
    Err(BlockfrostError::not_found())
}
