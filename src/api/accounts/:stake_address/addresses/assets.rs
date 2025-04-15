use crate::BlockfrostError;
use blockfrost_openapi;

pub async fn route() -> Result<HealthClockGet200Response, BlockfrostError> {
    Err(BlockfrostError::not_found())
}
