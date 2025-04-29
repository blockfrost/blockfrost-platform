use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::pool_updates_inner::PoolUpdatesInner;

pub async fn route() -> ApiResult<Vec<PoolUpdatesInner>> {
    Err(BlockfrostError::not_found())
}
