use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::DrepDelegatorsResponse;

pub async fn route() -> ApiResult<DrepDelegatorsResponse> {
    Err(BlockfrostError::not_found())
}
