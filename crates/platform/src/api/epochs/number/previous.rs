use bf_api_provider::types::EpochsStakeResponse;

use crate::{BlockfrostError, api::ApiResult};

pub async fn route() -> ApiResult<EpochsStakeResponse> {
    Err(BlockfrostError::not_found())
}
