use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::DrepsProposalVotesResponse;

pub async fn route() -> ApiResult<DrepsProposalVotesResponse> {
    Err(BlockfrostError::not_found())
}
