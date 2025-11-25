use crate::{BlockfrostError, api::ApiResult};
use bf_api_provider::types::DrepsVotesResponse;

pub async fn route() -> ApiResult<DrepsVotesResponse> {
    Err(BlockfrostError::not_found())
}
