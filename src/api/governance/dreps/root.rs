use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::dreps_inner::DrepsInner;

pub async fn route() -> ApiResult<Vec<DrepsInner>> {
    Err(BlockfrostError::not_found())
}
