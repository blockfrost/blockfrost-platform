use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::script_cbor::ScriptCbor;

pub async fn route() -> ApiResult<Vec<ScriptCbor>> {
    Err(BlockfrostError::not_found())
}
