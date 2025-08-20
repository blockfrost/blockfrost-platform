use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::TxsPoolCertsInnerRelaysResponse;

pub async fn route() -> ApiResult<TxsPoolCertsInnerRelaysResponse> {
    Err(BlockfrostError::not_found())
}
