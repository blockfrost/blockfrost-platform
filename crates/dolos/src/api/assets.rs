use crate::client::Dolos;
use blockfrost_openapi::models::asset::Asset;
use common::types::ApiResult;

impl Dolos {
    pub async fn assets_asset(&self, asset_id: &str) -> ApiResult<Asset> {
        let path = format!("assets/{asset_id}");

        self.client.get(&path, None).await
    }
}
