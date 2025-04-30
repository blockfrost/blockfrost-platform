use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::tx_content_delegations_inner::TxContentDelegationsInner;

pub async fn route() -> ApiResult<TxContentDelegationsInner> {
    Err(BlockfrostError::not_found())
}
