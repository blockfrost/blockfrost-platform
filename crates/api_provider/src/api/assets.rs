use crate::types::AssetsSingleResponse;
use async_trait::async_trait;
use common::{errors::BlockfrostError, types::ApiResult};

#[async_trait]
pub trait AssetsApi: Send + Sync + 'static {
    async fn assets_asset(&self, _asset_id: &str) -> ApiResult<AssetsSingleResponse> {
        Err(BlockfrostError::not_found())
    }
}
