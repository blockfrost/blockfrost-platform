use crate::{BlockfrostError, api::ApiResult};
use bf_api_provider::types::DrepsSingleProposalResponse;

pub async fn route() -> ApiResult<DrepsSingleProposalResponse> {
    Err(BlockfrostError::not_found())
}
