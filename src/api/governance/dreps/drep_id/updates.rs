use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::drep_updates_inner::DrepUpdatesInner;

pub async fn route() -> ApiResult<Vec<DrepUpdatesInner>> {
    Err(BlockfrostError::not_found())
}
