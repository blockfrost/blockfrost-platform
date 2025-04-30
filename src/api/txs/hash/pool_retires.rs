use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::tx_content_pool_retires_inner::TxContentPoolRetiresInner;

pub async fn route() -> ApiResult<Vec<TxContentPoolRetiresInner>> {
    Err(BlockfrostError::not_found())
}
