use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::proposal_votes_inner::ProposalVotesInner;

pub async fn route() -> ApiResult<Vec<ProposalVotesInner>> {
    Err(BlockfrostError::not_found())
}
