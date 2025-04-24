use crate::BlockfrostError;
use axum::Json;
use blockfrost_openapi::models::_health_clock_get_200_response::HealthClockGet200Response;

pub async fn route() -> Result<Json<HealthClockGet200Response>, BlockfrostError> {
    Err(BlockfrostError::not_found())
}
