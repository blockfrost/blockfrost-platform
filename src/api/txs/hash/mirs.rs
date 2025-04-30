use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::tx_content_mirs_inner::TxContentMirsInner;

pub async fn route() -> ApiResult<TxContentMirsInner> {
    Err(BlockfrostError::not_found())
}
