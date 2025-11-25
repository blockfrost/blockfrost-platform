use crate::{BlockfrostError, api::ApiResult};
use bf_api_provider::types::EpochsParamResponse;

pub async fn route() -> ApiResult<EpochsParamResponse> {
    Err(BlockfrostError::not_found())
}
