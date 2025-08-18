use crate::client::Dolos;
use api_provider::{
    api::txs::TxsApi,
    types::{
        TxsCborResponse, TxsDelegationsResponse, TxsMetadataCborResponse, TxsMetadataResponse,
        TxsMirsResponse, TxsPoolCertsResponse, TxsPoolRetiresResponse, TxsSingleResponse,
        TxsStakeAddrResponse, TxsUtxosResponse, TxsWithdrawalsResponse,
    },
};
use async_trait::async_trait;
use common::{pagination::Pagination, types::ApiResult};

#[async_trait]
impl TxsApi for Dolos {
    async fn txs_hash(&self, hash: &str) -> ApiResult<TxsSingleResponse> {
        let path = format!("txs/{hash}");

        self.client.get(&path, None).await
    }

    async fn txs_hash_cbor(&self, hash: &str) -> ApiResult<TxsCborResponse> {
        let path = format!("txs/{hash}/cbor");

        self.client.get(&path, None).await
    }

    async fn txs_hash_utxos(&self, hash: &str) -> ApiResult<TxsUtxosResponse> {
        let path = format!("txs/{hash}/utxos");

        self.client.get(&path, None).await
    }

    async fn txs_hash_metadata(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxsMetadataResponse> {
        let path = format!("txs/{hash}/metadata");

        self.client.get(&path, Some(pagination)).await
    }

    async fn txs_hash_metadata_cbor(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxsMetadataCborResponse> {
        let path = format!("txs/{hash}/metadata/cbor");

        self.client.get(&path, Some(pagination)).await
    }

    async fn txs_hash_withdrawals(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxsWithdrawalsResponse> {
        let path = format!("txs/{hash}/withdrawals");

        self.client.get(&path, Some(pagination)).await
    }

    async fn txs_hash_delegations(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxsDelegationsResponse> {
        let path = format!("txs/{hash}/delegations");

        self.client.get(&path, Some(pagination)).await
    }

    async fn txs_hash_mirs(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxsMirsResponse> {
        let path = format!("txs/{hash}/mirs");

        self.client.get(&path, Some(pagination)).await
    }

    async fn txs_hash_pool_updates(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxsPoolCertsResponse> {
        let path = format!("txs/{hash}/pool_updates");

        self.client.get(&path, Some(pagination)).await
    }

    async fn txs_hash_pool_retires(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxsPoolRetiresResponse> {
        let path = format!("txs/{hash}/pool_retires");

        self.client.get(&path, Some(pagination)).await
    }

    async fn txs_hash_stakes(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxsStakeAddrResponse> {
        let path = format!("txs/{hash}/stakes");

        self.client.get(&path, Some(pagination)).await
    }
}
