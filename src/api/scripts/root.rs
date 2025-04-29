use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::scripts_inner::ScriptsInner;

pub async fn route() -> ApiResult<Vec<ScriptsInner>> {
    Err(BlockfrostError::not_found())
}
