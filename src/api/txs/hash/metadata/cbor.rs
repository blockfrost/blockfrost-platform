use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::tx_content_metadata_cbor_inner::TxContentMetadataCborInner;

pub async fn route() -> ApiResult<TxContentMetadataCborInner> {
    Err(BlockfrostError::not_found())
}
