use crate::client::Dolos;
use api_provider::{api::addresses::AddressesApi, types::AddressUtxos};
use async_trait::async_trait;
use common::{pagination::Pagination, types::ApiResult};

#[async_trait]
impl AddressesApi for Dolos {
    async fn addresses_address_utxos(
        &self,
        address: &str,
        pagination: &Pagination,
    ) -> ApiResult<AddressUtxos> {
        let path = format!("addresses/{address}/utxos");

        self.client.get(&path, Some(pagination)).await
    }
}
