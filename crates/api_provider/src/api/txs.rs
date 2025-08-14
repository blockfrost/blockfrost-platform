use crate::types::{
    TxCborResponse, TxResponse, TxUtxosResponse, TxsDelegationsResponse, TxsMetadataCborResponse,
    TxsMetadataResponse, TxsMirsResponse, TxsPoolCertsResponse, TxsPoolRetiresResponse,
    TxsStakeAddrResponse, TxsWithdrawalsResponse,
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
    ) -> ApiResult<TxsMetadataResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn txs_hash_metadata_cbor(
        &self,
        _hash: &str,
        _pagination: &Pagination,
    ) -> ApiResult<TxsMetadataCborResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn txs_hash_withdrawals(
        &self,
        _hash: &str,
        _pagination: &Pagination,
    ) -> ApiResult<TxsWithdrawalsResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn txs_hash_delegations(
        &self,
        _hash: &str,
        _pagination: &Pagination,
    ) -> ApiResult<TxsDelegationsResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn txs_hash_mirs(
        &self,
        _hash: &str,
        _pagination: &Pagination,
    ) -> ApiResult<TxsMirsResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn txs_hash_pool_updates(
        &self,
        _hash: &str,
        _pagination: &Pagination,
    ) -> ApiResult<TxsPoolCertsResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn txs_hash_pool_retires(
        &self,
        _hash: &str,
        _pagination: &Pagination,
    ) -> ApiResult<TxsPoolRetiresResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn txs_hash_stakes(
        &self,
        _hash: &str,
        _pagination: &Pagination,
    ) -> ApiResult<TxsStakeAddrResponse> {
        Err(BlockfrostError::not_found())
    }
}
