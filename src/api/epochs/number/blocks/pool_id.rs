use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::epoch_stake_pool_content_inner::EpochStakePoolContentInner;

pub async fn route() -> ApiResult<Vec<EpochStakePoolContentInner>> {
    Err(BlockfrostError::not_found())
}
