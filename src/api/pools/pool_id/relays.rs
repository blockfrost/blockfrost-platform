use crate::{BlockfrostError, api::ApiResult};
use bf_api_provider::types::TxsPoolCertsInnerRelaysResponse;

pub async fn route() -> ApiResult<TxsPoolCertsInnerRelaysResponse> {
    Err(BlockfrostError::not_found())
}
