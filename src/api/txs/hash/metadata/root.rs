use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::tx_content_metadata_inner::TxContentMetadataInner;

pub async fn route() -> ApiResult<TxContentMetadataInner> {
    Err(BlockfrostError::not_found())
}
