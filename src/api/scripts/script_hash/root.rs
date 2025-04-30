use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::script::Script;

pub async fn route() -> ApiResult<Script> {
    Err(BlockfrostError::not_found())
}
