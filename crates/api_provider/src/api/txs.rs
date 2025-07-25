use crate::types::{
    TxCborResponse, TxDelegationsResponse, TxMetadataCborResponse, TxMetadataResponse,
    TxMirsResponse, TxPoolCertsResponse, TxPoolRetiresResponse, TxResponse, TxStakeAddrResponse,
    TxUtxosResponse, TxWithdrawalsResponse,
};
use async_trait::async_trait;
use common::{errors::BlockfrostError, pagination::Pagination, types::ApiResult};

#[async_trait]
pub trait TxsApi: Send + Sync + 'static {
    async fn txs_hash(&self, _hash: &str) -> ApiResult<TxResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn txs_hash_cbor(&self, _hash: &str) -> ApiResult<TxCborResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn txs_hash_utxos(&self, _hash: &str) -> ApiResult<TxUtxosResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn txs_hash_metadata(
        &self,
        _hash: &str,
        _pagination: &Pagination,
    ) -> ApiResult<TxMetadataResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn txs_hash_metadata_cbor(
        &self,
        _hash: &str,
        _pagination: &Pagination,
    ) -> ApiResult<TxMetadataCborResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn txs_hash_withdrawals(
        &self,
        _hash: &str,
        _pagination: &Pagination,
    ) -> ApiResult<TxWithdrawalsResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn txs_hash_delegations(
        &self,
        _hash: &str,
        _pagination: &Pagination,
    ) -> ApiResult<TxDelegationsResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn txs_hash_mirs(
        &self,
        _hash: &str,
        _pagination: &Pagination,
    ) -> ApiResult<TxMirsResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn txs_hash_pool_updates(
        &self,
        _hash: &str,
        _pagination: &Pagination,
    ) -> ApiResult<TxPoolCertsResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn txs_hash_pool_retires(
        &self,
        _hash: &str,
        _pagination: &Pagination,
    ) -> ApiResult<TxPoolRetiresResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn txs_hash_stakes(
        &self,
        _hash: &str,
        _pagination: &Pagination,
    ) -> ApiResult<TxStakeAddrResponse> {
        Err(BlockfrostError::not_found())
    }
}
