use crate::client::Dolos;
use blockfrost_openapi::models::{
    tx_content::TxContent, tx_content_cbor::TxContentCbor,
    tx_content_delegations_inner::TxContentDelegationsInner,
    tx_content_metadata_cbor_inner::TxContentMetadataCborInner,
    tx_content_metadata_inner::TxContentMetadataInner, tx_content_mirs_inner::TxContentMirsInner,
    tx_content_pool_certs_inner::TxContentPoolCertsInner,
    tx_content_pool_retires_inner::TxContentPoolRetiresInner,
    tx_content_stake_addr_inner::TxContentStakeAddrInner, tx_content_utxo::TxContentUtxo,
    tx_content_withdrawals_inner::TxContentWithdrawalsInner,
};
use common::{pagination::Pagination, types::ApiResult};

impl Dolos {
    pub async fn txs_hash(&self, hash: &str) -> ApiResult<TxContent> {
        let path = format!("txs/{hash}");

        self.client.get(&path, None).await
    }

    pub async fn txs_hash_cbor(&self, hash: &str) -> ApiResult<TxContentCbor> {
        let path = format!("txs/{hash}/cbor");

        self.client.get(&path, None).await
    }

    pub async fn txs_hash_utxos(&self, hash: &str) -> ApiResult<TxContentUtxo> {
        let path = format!("txs/{hash}/utxos");

        self.client.get(&path, None).await
    }

    pub async fn txs_hash_metadata(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<Vec<TxContentMetadataInner>> {
        let path = format!("txs/{hash}/metadata");

        self.client.get(&path, Some(pagination)).await
    }

    pub async fn txs_hash_metadata_cbor(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<Vec<TxContentMetadataCborInner>> {
        let path = format!("txs/{hash}/metadata/cbor");

        self.client.get(&path, Some(pagination)).await
    }

    pub async fn txs_hash_withdrawals(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<Vec<TxContentWithdrawalsInner>> {
        let path = format!("txs/{hash}/withdrawals");

        self.client.get(&path, Some(pagination)).await
    }

    pub async fn txs_hash_delegations(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<Vec<TxContentDelegationsInner>> {
        let path = format!("txs/{hash}/delegations");

        self.client.get(&path, Some(pagination)).await
    }

    pub async fn txs_hash_mirs(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<Vec<TxContentMirsInner>> {
        let path = format!("txs/{hash}/mirs");

        self.client.get(&path, Some(pagination)).await
    }

    pub async fn txs_hash_pool_updates(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<Vec<TxContentPoolCertsInner>> {
        let path = format!("txs/{hash}/pool_updates");

        self.client.get(&path, Some(pagination)).await
    }

    pub async fn txs_hash_pool_retires(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<Vec<TxContentPoolRetiresInner>> {
        let path = format!("txs/{hash}/pool_retires");

        self.client.get(&path, Some(pagination)).await
    }

    pub async fn txs_hash_stakes(
        &self,
        hash: &str,
        pagination: &Pagination,
    ) -> ApiResult<Vec<TxContentStakeAddrInner>> {
        let path = format!("txs/{hash}/stakes");

        self.client.get(&path, Some(pagination)).await
    }
}
