use crate::client::Dolos;
use bf_api_provider::types::{BlocksResponse, BlocksSingleResponse};
use bf_common::{pagination::Pagination, types::ApiResult};

pub struct DolosBlocks<'a> {
    pub(crate) inner: &'a Dolos,
}

impl Dolos {
    pub fn blocks(&self) -> DolosBlocks<'_> {
        DolosBlocks { inner: self }
    }
}

impl DolosBlocks<'_> {
    pub async fn latest(&self) -> ApiResult<BlocksSingleResponse> {
        self.inner.client.get("blocks/latest", None).await
    }

    pub async fn latest_txs(&self) -> ApiResult<Vec<String>> {
        self.inner.client.get("blocks/latest/txs", None).await
    }

    pub async fn by(&self, hash_or_number: &str) -> ApiResult<BlocksSingleResponse> {
        let path = format!("blocks/{hash_or_number}");

        self.inner.client.get(&path, None).await
    }

    pub async fn txs(
        &self,
        hash_or_number: &str,
        pagination: &Pagination,
    ) -> ApiResult<Vec<String>> {
        let path = format!("blocks/{hash_or_number}/txs");

        self.inner.client.get(&path, Some(pagination)).await
    }

    pub async fn previous(
        &self,
        hash_or_number: &str,
        pagination: &Pagination,
    ) -> ApiResult<BlocksResponse> {
        let path = format!("blocks/{hash_or_number}/previous");

        self.inner.client.get(&path, Some(pagination)).await
    }

    pub async fn next(
        &self,
        hash_or_number: &str,
        pagination: &Pagination,
    ) -> ApiResult<BlocksResponse> {
        let path = format!("blocks/{hash_or_number}/next");

        self.inner.client.get(&path, Some(pagination)).await
    }

    pub async fn by_slot(&self, slot: &str) -> ApiResult<BlocksSingleResponse> {
        let path = format!("blocks/slot/{slot}");

        self.inner.client.get(&path, None).await
    }
}
