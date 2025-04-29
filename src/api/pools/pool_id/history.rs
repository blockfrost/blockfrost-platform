use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::pool_history_inner::PoolHistoryInner;

pub async fn route() -> ApiResult<Vec<PoolHistoryInner>> {
    Err(BlockfrostError::not_found())
}
