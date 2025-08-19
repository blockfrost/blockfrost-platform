use crate::client::Dolos;
use api_provider::types::AssetsSingleResponse;
use common::types::ApiResult;

pub struct DolosAssets<'a> {
    pub(crate) inner: &'a Dolos,
}

impl Dolos {
    pub fn assets(&self) -> DolosAssets<'_> {
        DolosAssets { inner: self }
    }
}

impl DolosAssets<'_> {
    pub async fn asset(&self, asset_id: &str) -> ApiResult<AssetsSingleResponse> {
        let path = format!("assets/{asset_id}");

        self.inner.client.get(&path, None).await
    }
}
