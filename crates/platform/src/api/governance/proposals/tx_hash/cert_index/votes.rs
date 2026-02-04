use crate::{BlockfrostError, api::ApiResult};
use bf_api_provider::types::DrepsProposalVotesResponse;

pub async fn route() -> ApiResult<DrepsProposalVotesResponse> {
    Err(BlockfrostError::not_found())
}
