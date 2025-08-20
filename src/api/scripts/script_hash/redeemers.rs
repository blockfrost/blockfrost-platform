use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::ScriptsRedeemersInnerResponse;

pub async fn route() -> ApiResult<ScriptsRedeemersInnerResponse> {
    Err(BlockfrostError::not_found())
}
