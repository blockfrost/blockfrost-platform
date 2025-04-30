use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::pool_list_extended_inner::PoolListExtendedInner;

pub async fn route() -> ApiResult<Vec<PoolListExtendedInner>> {
    Err(BlockfrostError::not_found())
}
