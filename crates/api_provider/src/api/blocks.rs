use crate::types::{BlocksResponse, BlocksSingleResponse};
use async_trait::async_trait;
use common::{errors::BlockfrostError, pagination::Pagination, types::ApiResult};

#[async_trait]
pub trait BlocksApi: Send + Sync + 'static {
    async fn blocks_latest(&self) -> ApiResult<BlocksSingleResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn blocks_latest_txs(&self) -> ApiResult<Vec<String>> {
        Err(BlockfrostError::not_found())
    }

    async fn blocks_hash_or_number(
        &self,
        _hash_or_number: &str,
    ) -> ApiResult<BlocksSingleResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn blocks_hash_or_number_txs(
        &self,
        _hash_or_number: &str,
        _pagination: &Pagination,
    ) -> ApiResult<Vec<String>> {
        Err(BlockfrostError::not_found())
    }

    async fn blocks_previous(
        &self,
        _hash_or_number: &str,
        _pagination: &Pagination,
    ) -> ApiResult<BlocksResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn blocks_next(
        &self,
        _hash_or_number: &str,
        _pagination: &Pagination,
    ) -> ApiResult<BlocksResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn blocks_slot_slot(&self, _slot: &str) -> ApiResult<BlocksSingleResponse> {
        Err(BlockfrostError::not_found())
    }
}
