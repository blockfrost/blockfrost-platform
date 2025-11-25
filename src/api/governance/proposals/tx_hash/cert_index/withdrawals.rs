use crate::{BlockfrostError, api::ApiResult};
use bf_api_provider::types::DrepsProposalWithdrawalsResponse;

pub async fn route() -> ApiResult<DrepsProposalWithdrawalsResponse> {
    Err(BlockfrostError::not_found())
}
