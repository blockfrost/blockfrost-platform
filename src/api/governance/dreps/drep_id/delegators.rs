use bf_api_provider::types::DrepsDelegatorsResponse;

use crate::{BlockfrostError, api::ApiResult};

pub async fn route() -> ApiResult<DrepsDelegatorsResponse> {
    Err(BlockfrostError::not_found())
}
