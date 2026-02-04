use crate::{BlockfrostError, api::ApiResult};
use bf_api_provider::types::PoolsHistoryResponse;

pub async fn route() -> ApiResult<PoolsHistoryResponse> {
    Err(BlockfrostError::not_found())
}
