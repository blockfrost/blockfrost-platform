use crate::client::DataNode;
use bf_api_provider::types::{
    AssetsAddressesResponse, AssetsSingleResponse, AssetsTransactionsResponse,
};
use bf_common::{pagination::Pagination, types::ApiResult};

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

    pub async fn addresses(
        &self,
        asset_id: &str,
        pagination: &Pagination,
    ) -> ApiResult<AssetsAddressesResponse> {
        let path = format!("assets/{asset_id}/addresses");

        self.inner.client.get(&path, Some(pagination)).await
    }

    pub async fn transactions(
        &self,
        asset_id: &str,
        pagination: &Pagination,
    ) -> ApiResult<AssetsTransactionsResponse> {
        let path = format!("assets/{asset_id}/transactions");

        self.inner.client.get(&path, Some(pagination)).await
    }
}
