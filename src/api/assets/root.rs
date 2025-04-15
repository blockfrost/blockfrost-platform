use blockfrost_openapi::models::assets_inner::AssetsInner;

use crate::BlockfrostError;

pub async fn route() -> Result<Vec<AssetsInner>, BlockfrostError> {
    Err(BlockfrostError::not_found())
}
