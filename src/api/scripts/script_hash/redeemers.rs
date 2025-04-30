use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::script_redeemers_inner::ScriptRedeemersInner;

pub async fn route() -> ApiResult<Vec<ScriptRedeemersInner>> {
    Err(BlockfrostError::not_found())
}
