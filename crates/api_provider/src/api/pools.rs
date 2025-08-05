use crate::types::{PoolDelegatorsResponse, PoolListExtendedResponse};
use async_trait::async_trait;
use common::{errors::BlockfrostError, pagination::Pagination, types::ApiResult};

#[async_trait]
pub trait PoolsApi: Send + Sync + 'static {
    async fn pools_extended(&self) -> ApiResult<PoolListExtendedResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn pools_pool_id_delegators(
        &self,
        _pool_id: &str,
        _pagination: &Pagination,
    ) -> ApiResult<PoolDelegatorsResponse> {
        Err(BlockfrostError::not_found())
    }
}
