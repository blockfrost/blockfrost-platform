use crate::client::Dolos;
use api_provider::types::{
    AddressesTransactionsResponse, AddressesUtxosAssetResponse, AddressesUtxosResponse,
};
use common::{pagination::Pagination, types::ApiResult};

pub struct DolosAddresses<'a> {
    pub(crate) inner: &'a Dolos,
}

impl Dolos {
    pub fn addresses(&self) -> DolosAddresses<'_> {
        DolosAddresses { inner: self }
    }
}

impl DolosAddresses<'_> {
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
