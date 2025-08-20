use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::ScriptsInnerResponse;

pub async fn route() -> ApiResult<ScriptsInnerResponse> {
    Err(BlockfrostError::not_found())
}
