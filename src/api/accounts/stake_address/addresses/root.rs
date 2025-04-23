use crate::BlockfrostError;
use axum::Json;
use blockfrost_openapi::models::network_eras_inner::NetworkErasInner;

pub async fn route() -> Result<Json<Vec<NetworkErasInner>>, BlockfrostError> {
    Err(BlockfrostError::not_found())
}
