use crate::{BlockfrostError, api::ApiResult};

pub async fn route() -> ApiResult<Vec<String>> {
    Err(BlockfrostError::not_found())
}
