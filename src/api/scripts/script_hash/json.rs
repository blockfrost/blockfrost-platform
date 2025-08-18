use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::ScriptsJsonResponse;

pub async fn route() -> ApiResult<ScriptsJsonResponse> {
    Err(BlockfrostError::not_found())
}
