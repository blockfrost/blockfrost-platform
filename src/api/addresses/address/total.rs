use crate::{BlockfrostError, api::ApiResult};

pub async fn route() -> ApiResult<()> {
    Err(BlockfrostError::not_found())
}
