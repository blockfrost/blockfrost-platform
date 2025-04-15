use crate::BlockfrostError;
use blockfrost_openapi::models::_health_get_200_response::HealthGet200Response;

pub async fn route() -> Result<HealthGet200Response, BlockfrostError> {
    Err(BlockfrostError::not_found())
}
