use crate::client::Dolos;
use api_provider::{
    api::txs::TxsApi,
    types::{
        TxCborResponse, TxDelegationsResponse, TxMetadataCborResponse, TxMetadataResponse,
        TxMirsResponse, TxPoolCertsResponse, TxPoolRetiresResponse, TxResponse,
        TxStakeAddrResponse, TxUtxosResponse, TxWithdrawalsResponse,
    },
};
use async_trait::async_trait;
use common::{pagination::Pagination, types::ApiResult};

#[async_trait]
impl TxsApi for Dolos {
    async fn txs_hash(&self, hash: &str) -> ApiResult<TxResponse> {
        let path = format!("txs/{hash}");

        self.client.get(&path, None).await
    }

    async fn txs_hash_cbor(&self, hash: &str) -> ApiResult<TxCborResponse> {
        let path = format!("txs/{hash}/cbor");

        self.client.get(&path, None).await
    }

    async fn txs_hash_utxos(&self, hash: &str) -> ApiResult<TxUtxosResponse> {
        let path = format!("txs/{hash}/utxos");

        self.client.get(&path, None).await
    }

    async fn txs_hash_metadata(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxMetadataResponse> {
        let path = format!("txs/{hash}/metadata");

        self.client.get(&path, Some(pagination)).await
    }

    async fn txs_hash_metadata_cbor(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxMetadataCborResponse> {
        let path = format!("txs/{hash}/metadata/cbor");

        self.client.get(&path, Some(pagination)).await
    }

    async fn txs_hash_withdrawals(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxWithdrawalsResponse> {
        let path = format!("txs/{hash}/withdrawals");

        self.client.get(&path, Some(pagination)).await
    }

    async fn txs_hash_delegations(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxDelegationsResponse> {
        let path = format!("txs/{hash}/delegations");

        self.client.get(&path, Some(pagination)).await
    }

    async fn txs_hash_mirs(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxMirsResponse> {
        let path = format!("txs/{hash}/mirs");

        self.client.get(&path, Some(pagination)).await
    }

    async fn txs_hash_pool_updates(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxPoolCertsResponse> {
        let path = format!("txs/{hash}/pool_updates");

        self.client.get(&path, Some(pagination)).await
    }

    async fn txs_hash_pool_retires(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxPoolRetiresResponse> {
        let path = format!("txs/{hash}/pool_retires");

        self.client.get(&path, Some(pagination)).await
    }

    async fn txs_hash_stakes(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxStakeAddrResponse> {
        let path = format!("txs/{hash}/stakes");

        self.client.get(&path, Some(pagination)).await
    }
}
