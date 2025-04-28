use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::drep_votes_inner::DrepVotesInner;

pub async fn route() -> ApiResult<Vec<DrepVotesInner>> {
    Err(BlockfrostError::not_found())
}
