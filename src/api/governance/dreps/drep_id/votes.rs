use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::DrepVotesResponse;

pub async fn route() -> ApiResult<DrepVotesResponse> {
    Err(BlockfrostError::not_found())
}
