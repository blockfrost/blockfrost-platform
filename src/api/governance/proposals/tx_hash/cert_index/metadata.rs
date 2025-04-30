use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::drep_metadata::DrepMetadata;

pub async fn route() -> ApiResult<DrepMetadata> {
    Err(BlockfrostError::not_found())
}
