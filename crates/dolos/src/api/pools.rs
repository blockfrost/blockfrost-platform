use crate::client::Dolos;
use api_provider::{
    api::pools::PoolsApi,
    types::{PoolDelegatorsResponse, PoolListExtendedResponse},
};
use async_trait::async_trait;
use common::{pagination::Pagination, types::ApiResult};

#[async_trait]
impl PoolsApi for Dolos {
    async fn pools_extended(&self) -> ApiResult<PoolListExtendedResponse> {
        self.client.get("pools/extended", None).await
    }

    async fn pools_pool_id_delegators(
        &self,
        pool_id: &str,
        pagination: &Pagination,
    ) -> ApiResult<PoolDelegatorsResponse> {
        let path = format!("pools/{pool_id}/delegators");

        self.client.get(&path, Some(pagination)).await
    }
}
