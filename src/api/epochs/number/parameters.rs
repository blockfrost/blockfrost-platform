use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::epoch_param_content::EpochParamContent;

pub async fn route() -> ApiResult<EpochParamContent> {
    Err(BlockfrostError::not_found())
}
