use crate::{BlockfrostError, api::ApiResult};
use bf_api_provider::types::PoolsProposalVotesResponse;

pub async fn route() -> ApiResult<PoolsProposalVotesResponse> {
    Err(BlockfrostError::not_found())
}
