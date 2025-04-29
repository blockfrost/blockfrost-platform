use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::pool::Pool;

pub async fn route() -> ApiResult<Vec<Pool>> {
    Err(BlockfrostError::not_found())
}
