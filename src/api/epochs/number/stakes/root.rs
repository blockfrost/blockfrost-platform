use api_provider::types::EpochStakePoolResponse;

use crate::{BlockfrostError, api::ApiResult};

pub async fn route() -> ApiResult<EpochStakePoolResponse> {
    Err(BlockfrostError::not_found())
}
