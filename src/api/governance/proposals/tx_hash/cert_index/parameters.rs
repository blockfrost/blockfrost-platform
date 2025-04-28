use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::proposal_parameters_parameters::ProposalParametersParameters;

pub async fn route() -> ApiResult<ProposalParametersParameters> {
    Err(BlockfrostError::not_found())
}
