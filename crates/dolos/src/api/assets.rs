use crate::client::Dolos;
use api_provider::{api::assets::AssetsApi, types::AssetsSingleResponse};
use async_trait::async_trait;
use common::types::ApiResult;

#[async_trait]
impl AssetsApi for Dolos {
    async fn assets_asset(&self, asset_id: &str) -> ApiResult<AssetsSingleResponse> {
        let path = format!("assets/{asset_id}");

        self.client.get(&path, None).await
    }
}
