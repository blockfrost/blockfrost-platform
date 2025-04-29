use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::tx_content_stake_addr_inner::TxContentStakeAddrInner;

pub async fn route() -> ApiResult<TxContentStakeAddrInner> {
    Err(BlockfrostError::not_found())
}
