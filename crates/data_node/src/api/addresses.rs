use crate::client::DataNode;
use bf_api_provider::types::{
    AddressesTransactionsResponse, AddressesUtxosAssetResponse, AddressesUtxosResponse,
};
use bf_common::{pagination::Pagination, types::ApiResult};

pub struct DataNodeAddresses<'a> {
    pub(crate) inner: &'a DataNode,
}

impl DataNode {
    pub fn addresses(&self) -> DataNodeAddresses<'_> {
        DataNodeAddresses { inner: self }
    }
}

impl DataNodeAddresses<'_> {
    pub async fn utxos(
        &self,
        address: &str,
        pagination: &Pagination,
    ) -> ApiResult<AddressesUtxosResponse> {
        let path = format!("addresses/{address}/utxos");

        self.inner.client.get(&path, Some(pagination)).await
    }

    pub async fn utxos_asset(
        &self,
        address: &str,
        asset: &str,
        pagination: &Pagination,
    ) -> ApiResult<AddressesUtxosAssetResponse> {
        let path = format!("addresses/{address}/utxos/{asset}");

        self.inner.client.get(&path, Some(pagination)).await
    }

    pub async fn transactions(
        &self,
        address: &str,
        pagination: &Pagination,
    ) -> ApiResult<AddressesTransactionsResponse> {
        let path = format!("addresses/{address}/transactions");

        self.inner.client.get(&path, Some(pagination)).await
    }
}
