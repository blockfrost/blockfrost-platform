use bf_api_provider::types::DrepsProposalsResponse;

use crate::{BlockfrostError, api::ApiResult};

pub async fn route() -> ApiResult<DrepsProposalsResponse> {
    Err(BlockfrostError::not_found())
}
