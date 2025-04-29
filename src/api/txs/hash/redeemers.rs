use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::tx_content_redeemers_inner::TxContentRedeemersInner;

pub async fn route() -> ApiResult<TxContentRedeemersInner> {
    Err(BlockfrostError::not_found())
}
