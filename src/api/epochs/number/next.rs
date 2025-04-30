use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::epoch_stake_content_inner::EpochStakeContentInner;

pub async fn route() -> ApiResult<Vec<EpochStakeContentInner>> {
    Err(BlockfrostError::not_found())
}
