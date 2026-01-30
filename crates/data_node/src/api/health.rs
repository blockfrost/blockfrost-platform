use crate::client::DataNode;
use bf_api_provider::types::HealthResponse;
use bf_common::types::ApiResult;

pub struct DataNodeHealth<'a> {
    pub(crate) inner: &'a DataNode,
}

impl DataNode {
    pub fn health(&self) -> DataNodeHealth<'_> {
        DataNodeHealth { inner: self }
    }
}

impl DataNodeHealth<'_> {
    pub async fn get(&self) -> ApiResult<HealthResponse> {
        self.inner.client.get("health", None).await
    }
}
