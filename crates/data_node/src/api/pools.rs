use crate::client::DataNode;
use bf_api_provider::types::{
    PoolsDelegatorsResponse, PoolsListExtendedResponse, PoolsMetadataResponse, PoolsSingleResponse,
};
use bf_common::{pagination::Pagination, types::ApiResult};

pub struct DataNodePools<'a> {
    pub(crate) inner: &'a DataNode,
}

impl DataNode {
    pub fn pools(&self) -> DataNodePools<'_> {
        DataNodePools { inner: self }
    }
}

impl DataNodePools<'_> {
    pub async fn extended(&self, pagination: &Pagination) -> ApiResult<PoolsListExtendedResponse> {
        self.inner
            .client
            .get("pools/extended", Some(pagination))
            .await
    }

    pub async fn delegators(
        &self,
        pool_id: &str,
        pagination: &Pagination,
    ) -> ApiResult<PoolsDelegatorsResponse> {
        let path = format!("pools/{pool_id}/delegators");

        self.inner.client.get(&path, Some(pagination)).await
    }

    pub async fn by_id(&self, pool_id: &str) -> ApiResult<PoolsSingleResponse> {
        let path = format!("pools/{pool_id}");

        self.inner.client.get(&path, None).await
    }

    pub async fn metadata(&self, pool_id: &str) -> ApiResult<PoolsMetadataResponse> {
        let path = format!("pools/{pool_id}/metadata");

        self.inner.client.get(&path, None).await
    }
}
