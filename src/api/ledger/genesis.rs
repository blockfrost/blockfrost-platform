use crate::BlockfrostError;
use axum::Json;
use blockfrost_openapi::models::genesis_content::GenesisContent;

pub async fn route() -> Result<Json<GenesisContent>, BlockfrostError> {
    Err(BlockfrostError::not_found())
}
