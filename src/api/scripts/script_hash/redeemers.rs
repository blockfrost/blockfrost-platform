use crate::{BlockfrostError, api::ApiResult};
use bf_api_provider::types::ScriptsRedeemersInnerResponse;

pub async fn route() -> ApiResult<ScriptsRedeemersInnerResponse> {
    Err(BlockfrostError::not_found())
}
