use crate::api::types::AddressUtxos;
use async_trait::async_trait;
use common::{errors::BlockfrostError, pagination::Pagination, types::ApiResult};

#[async_trait]
pub trait AddressesApi: Send + Sync + 'static {
    async fn addresses_address_utxos(
        &self,
        _address: &str,
        _pagination: &Pagination,
    ) -> ApiResult<AddressUtxos> {
        Err(BlockfrostError::not_found())
    }
}
