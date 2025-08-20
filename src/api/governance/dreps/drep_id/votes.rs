use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::DrepsVotesResponse;

pub async fn route() -> ApiResult<DrepsVotesResponse> {
    Err(BlockfrostError::not_found())
}
