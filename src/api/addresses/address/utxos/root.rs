use crate::BlockfrostError;
use axum::Json;
use blockfrost_openapi::models::assets_inner::AssetsInner;

pub async fn route() -> Result<Json<Vec<AssetsInner>>, BlockfrostError> {
    Err(BlockfrostError::not_found())
}
