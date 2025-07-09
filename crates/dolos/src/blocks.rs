use crate::client::Dolos;
use blockfrost_openapi::models::block_content::BlockContent;
use common::{pagination::Pagination, types::ApiResult};

impl Dolos {
    pub async fn blocks_latest(&self) -> ApiResult<BlockContent> {
        self.client.get("blocks/latest").await
    }

    pub async fn blocks_latest_txs(&self) -> ApiResult<Vec<String>> {
        self.client.get("blocks/latest/txs").await
    }

    pub async fn blocks_hash_or_number(&self, hash_or_number: &str) -> ApiResult<BlockContent> {
        let path = &format!("blocks/{hash_or_number}");

        self.client.get(path).await
    }

    pub async fn blocks_hash_or_number_txs(
        &self,
        hash_or_number: &str,
        pagination: Pagination,
    ) -> ApiResult<Vec<String>> {
        let path = &format!("blocks/{hash_or_number}/txs");

        self.client.get_paginated(path, &pagination).await
    }

    pub async fn blocks_previous(
        &self,
        hash_or_number: &str,
        pagination: Pagination,
    ) -> ApiResult<Vec<BlockContent>> {
        let path = &format!("blocks/{hash_or_number}/previous");

        self.client.get_paginated(path, &pagination).await
    }

    pub async fn blocks_next(
        &self,
        hash_or_number: &str,
        pagination: Pagination,
    ) -> ApiResult<Vec<BlockContent>> {
        let path = &format!("blocks/{hash_or_number}/next");

        self.client.get_paginated(path, &pagination).await
    }

    pub async fn blocks_slot_slot(&self, slot: &str) -> ApiResult<BlockContent> {
        let path = &format!("blocks/slot/{slot}");

        self.client.get(path).await
    }
}
