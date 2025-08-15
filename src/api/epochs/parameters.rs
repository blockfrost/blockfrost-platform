use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::EpochsParamResponse;

pub async fn route() -> ApiResult<EpochsParamResponse> {
    Err(BlockfrostError::not_found())
}
