use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::tx_content_withdrawals_inner::TxContentWithdrawalsInner;

pub async fn route() -> ApiResult<TxContentWithdrawalsInner> {
    Err(BlockfrostError::not_found())
}
