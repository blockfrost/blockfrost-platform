use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::drep_delegators_inner::DrepDelegatorsInner;

pub async fn route() -> ApiResult<Vec<DrepDelegatorsInner>> {
    Err(BlockfrostError::not_found())
}
