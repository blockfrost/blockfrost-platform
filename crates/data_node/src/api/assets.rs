use crate::client::DataNode;
use bf_api_provider::types::AssetsSingleResponse;
use bf_common::types::ApiResult;

pub struct DataNodeAssets<'a> {
    pub(crate) inner: &'a DataNode,
}

impl DataNode {
    pub fn assets(&self) -> DataNodeAssets<'_> {
        DataNodeAssets { inner: self }
    }
}

impl DataNodeAssets<'_> {
    pub async fn asset(&self, asset_id: &str) -> ApiResult<AssetsSingleResponse> {
        let path = format!("assets/{asset_id}");

        self.inner.client.get(&path, None).await
    }
}
