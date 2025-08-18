use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::DrepsProposalWithdrawalsResponse;

pub async fn route() -> ApiResult<DrepsProposalWithdrawalsResponse> {
    Err(BlockfrostError::not_found())
}
