use crate::{BlockfrostError, api::ApiResult};
use cardano_serialization_lib::ScriptDataHash;

pub async fn route() -> ApiResult<Vec<ScriptDataHash>> {
    Err(BlockfrostError::not_found())
}
