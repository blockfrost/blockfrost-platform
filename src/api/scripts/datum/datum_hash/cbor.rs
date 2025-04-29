use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::script_datum_cbor::ScriptDatumCbor;

pub async fn route() -> ApiResult<Vec<ScriptDatumCbor>> {
    Err(BlockfrostError::not_found())
}
