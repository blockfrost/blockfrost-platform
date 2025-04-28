use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::proposal::Proposal;

pub async fn route() -> ApiResult<Proposal> {
    Err(BlockfrostError::not_found())
}
