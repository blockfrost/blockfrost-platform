use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::tx_content_pool_certs_inner_relays_inner::TxContentPoolCertsInnerRelaysInner;

pub async fn route() -> ApiResult<Vec<TxContentPoolCertsInnerRelaysInner>> {
    Err(BlockfrostError::not_found())
}
