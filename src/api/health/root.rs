use crate::BlockfrostError;
use axum::Json;
use blockfrost_openapi::models::_health_get_200_response::HealthGet200Response;

pub async fn route() -> Result<Json<HealthGet200Response>, BlockfrostError> {
    Err(BlockfrostError::not_found())
}
