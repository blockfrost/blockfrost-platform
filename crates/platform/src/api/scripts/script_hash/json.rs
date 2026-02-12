use crate::{BlockfrostError, api::ApiResult};
use bf_api_provider::types::ScriptsJsonResponse;

pub async fn route() -> ApiResult<ScriptsJsonResponse> {
    Err(BlockfrostError::not_found())
}
