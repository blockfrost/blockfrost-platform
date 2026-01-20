use crate::client::DataNode;
use bf_api_provider::types::{
    TxsCborResponse, TxsDelegationsResponse, TxsMetadataCborResponse, TxsMetadataResponse,
    TxsMirsResponse, TxsPoolCertsResponse, TxsPoolRetiresResponse, TxsRedeemersResponse,
    TxsSingleResponse, TxsStakeAddrResponse, TxsUtxosResponse, TxsWithdrawalsResponse,
};
use bf_common::{pagination::Pagination, types::ApiResult};

pub struct DataNodeTxs<'a> {
    pub(crate) inner: &'a DataNode,
}

impl DataNode {
    pub fn txs(&self) -> DataNodeTxs<'_> {
        DataNodeTxs { inner: self }
    }
}

impl DataNodeTxs<'_> {
    pub async fn by_hash(&self, hash: &str) -> ApiResult<TxsSingleResponse> {
        let path = format!("txs/{hash}");

        self.inner.client.get(&path, None).await
    }

    pub async fn cbor(&self, hash: &str) -> ApiResult<TxsCborResponse> {
        let path = format!("txs/{hash}/cbor");

        self.inner.client.get(&path, None).await
    }

    pub async fn utxos(&self, hash: &str, pagination: &Pagination) -> ApiResult<TxsUtxosResponse> {
        let path = format!("txs/{hash}/utxos");

        self.inner.client.get(&path, Some(pagination)).await
    }

    pub async fn metadata(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxsMetadataResponse> {
        let path = format!("txs/{hash}/metadata");

        self.inner.client.get(&path, Some(pagination)).await
    }

    pub async fn metadata_cbor(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxsMetadataCborResponse> {
        let path = format!("txs/{hash}/metadata/cbor");

        self.inner.client.get(&path, Some(pagination)).await
    }

    pub async fn withdrawals(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxsWithdrawalsResponse> {
        let path = format!("txs/{hash}/withdrawals");

        self.inner.client.get(&path, Some(pagination)).await
    }

    pub async fn delegations(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxsDelegationsResponse> {
        let path = format!("txs/{hash}/delegations");

        self.inner.client.get(&path, Some(pagination)).await
    }

    pub async fn mirs(&self, hash: &str, pagination: &Pagination) -> ApiResult<TxsMirsResponse> {
        let path = format!("txs/{hash}/mirs");

        self.inner.client.get(&path, Some(pagination)).await
    }

    pub async fn redeemers(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxsRedeemersResponse> {
        let path = format!("txs/{hash}/redeemers");

        self.inner.client.get(&path, Some(pagination)).await
    }

    pub async fn pool_updates(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxsPoolCertsResponse> {
        let path = format!("txs/{hash}/pool_updates");
        self.inner.client.get(&path, Some(pagination)).await
    }

    pub async fn pool_retires(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxsPoolRetiresResponse> {
        let path = format!("txs/{hash}/pool_retires");
        self.inner.client.get(&path, Some(pagination)).await
    }

    pub async fn stakes(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<TxsStakeAddrResponse> {
        let path = format!("txs/{hash}/stakes");
        self.inner.client.get(&path, Some(pagination)).await
    }
}
