use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::tx_content_required_signers_inner::TxContentRequiredSignersInner;

pub async fn route() -> ApiResult<TxContentRequiredSignersInner> {
    Err(BlockfrostError::not_found())
}
