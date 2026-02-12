use bf_api_provider::types::DrepsProposalParametersResponse;

use crate::{BlockfrostError, api::ApiResult};

pub async fn route() -> ApiResult<DrepsProposalParametersResponse> {
    Err(BlockfrostError::not_found())
}
