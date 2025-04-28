use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::proposal_withdrawals_inner::ProposalWithdrawalsInner;

pub async fn route() -> ApiResult<Vec<ProposalWithdrawalsInner>> {
    Err(BlockfrostError::not_found())
}
