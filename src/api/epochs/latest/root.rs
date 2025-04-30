use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::epoch_content::EpochContent;

pub async fn route() -> ApiResult<EpochContent> {
    Err(BlockfrostError::not_found())
}
