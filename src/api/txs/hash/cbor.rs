use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::tx_content_cbor::TxContentCbor;

pub async fn route() -> ApiResult<TxContentCbor> {
    Err(BlockfrostError::not_found())
}
