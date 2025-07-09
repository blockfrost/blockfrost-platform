use crate::client::Dolos;
use blockfrost_openapi::models::address_utxo_content_inner::AddressUtxoContentInner;
use common::{pagination::Pagination, types::ApiResult};

impl Dolos {
    pub async fn addresses_address_utxos(
        &self,
        address: &str,
        pagination: &Pagination,
    ) -> ApiResult<Vec<AddressUtxoContentInner>> {
        let path = &format!("addresses/{address}/utxos");

        self.client.get_paginated(path, pagination).await
    }
}
