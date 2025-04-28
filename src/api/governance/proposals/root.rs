use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::proposals_inner::ProposalsInner;

pub async fn route() -> ApiResult<Vec<ProposalsInner>> {
    Err(BlockfrostError::not_found())
}
