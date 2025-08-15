use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::EpochsStakeResponse;

pub async fn route() -> ApiResult<EpochsStakeResponse> {
    Err(BlockfrostError::not_found())
}
