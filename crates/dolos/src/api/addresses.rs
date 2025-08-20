use crate::client::Dolos;
use api_provider::types::AddressesUtxosResponse;
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
}
