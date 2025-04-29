use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::pool_delegators_inner::PoolDelegatorsInner;

pub async fn route() -> ApiResult<Vec<PoolDelegatorsInner>> {
    Err(BlockfrostError::not_found())
}
