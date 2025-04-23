use crate::BlockfrostError;
use blockfrost_openapi::models::assets_inner::AssetsInner;

pub async fn route() -> Result<Vec<AssetsInner>, BlockfrostError> {
    Err(BlockfrostError::not_found())
}
