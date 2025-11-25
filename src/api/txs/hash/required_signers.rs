use crate::{BlockfrostError, api::ApiResult};
use bf_api_provider::types::TxsContentRequiredSignersInner;

pub async fn route() -> ApiResult<TxsContentRequiredSignersInner> {
    Err(BlockfrostError::not_found())
}
