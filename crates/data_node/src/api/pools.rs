use crate::client::DataNode;
use bf_api_provider::types::{PoolsDelegatorsResponse, PoolsListExtendedResponse};
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
}
