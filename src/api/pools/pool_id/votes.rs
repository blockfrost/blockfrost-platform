use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::PoolsProposalVotesResponse;

pub async fn route() -> ApiResult<PoolsProposalVotesResponse> {
    Err(BlockfrostError::not_found())
}
