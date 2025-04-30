use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::script_json::ScriptJson;

pub async fn route() -> ApiResult<Vec<ScriptJson>> {
    Err(BlockfrostError::not_found())
}
