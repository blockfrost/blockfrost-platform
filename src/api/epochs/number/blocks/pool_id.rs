use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::EpochStakePoolResponse;

pub async fn route() -> ApiResult<EpochStakePoolResponse> {
    Err(BlockfrostError::not_found())
}
