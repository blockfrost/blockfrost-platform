use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::pool_list_retire_inner::PoolListRetireInner;

pub async fn route() -> ApiResult<Vec<PoolListRetireInner>> {
    Err(BlockfrostError::not_found())
}
