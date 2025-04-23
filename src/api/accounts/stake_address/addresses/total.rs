use crate::BlockfrostError;
use blockfrost_openapi::models::_health_clock_get_200_response::HealthClockGet200Response;

pub async fn route() -> Result<HealthClockGet200Response, BlockfrostError> {
    Err(BlockfrostError::not_found())
}
