use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::TxsRedeemersResponse;

pub async fn route() -> ApiResult<TxsRedeemersResponse> {
    Err(BlockfrostError::not_found())
}
