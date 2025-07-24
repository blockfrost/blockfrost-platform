use crate::client::Dolos;
use api_provider::{api::blocks::BlocksApi, types::BlockResponse};
use async_trait::async_trait;
use common::{pagination::Pagination, types::ApiResult};

#[async_trait]
impl BlocksApi for Dolos {
    async fn blocks_latest(&self) -> ApiResult<BlockResponse> {
        self.client.get("blocks/latest", None).await
    }

    async fn blocks_latest_txs(&self) -> ApiResult<Vec<String>> {
        self.client.get("blocks/latest/txs", None).await
    }

    async fn blocks_hash_or_number(&self, hash_or_number: &str) -> ApiResult<BlockResponse> {
        let path = format!("blocks/{hash_or_number}");

        self.client.get(&path, None).await
    }

    async fn blocks_hash_or_number_txs(
        &self,
        hash_or_number: &str,
        pagination: &Pagination,
    ) -> ApiResult<Vec<String>> {
        let path = format!("blocks/{hash_or_number}/txs");

        self.client.get(&path, Some(pagination)).await
    }

    async fn blocks_previous(
        &self,
        hash_or_number: &str,
        pagination: &Pagination,
    ) -> ApiResult<Vec<BlockResponse>> {
        let path = format!("blocks/{hash_or_number}/previous");

        self.client.get(&path, Some(pagination)).await
    }

    async fn blocks_next(
        &self,
        hash_or_number: &str,
        pagination: &Pagination,
    ) -> ApiResult<Vec<BlockResponse>> {
        let path = format!("blocks/{hash_or_number}/next");

        self.client.get(&path, Some(pagination)).await
    }

    async fn blocks_slot_slot(&self, slot: &str) -> ApiResult<BlockResponse> {
        let path = format!("blocks/slot/{slot}");

        self.client.get(&path, None).await
    }
}
