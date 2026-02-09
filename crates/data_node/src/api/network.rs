use crate::client::DataNode;
use bf_api_provider::types::{NetworkErasResponse, NetworkResponse};
use bf_common::types::ApiResult;

pub struct DataNodeNetwork<'a> {
    pub(crate) inner: &'a DataNode,
}

impl DataNode {
    pub fn network(&self) -> DataNodeNetwork<'_> {
        DataNodeNetwork { inner: self }
    }
}

impl DataNodeNetwork<'_> {
    pub async fn get(&self) -> ApiResult<NetworkResponse> {
        self.inner.client.get("network", None).await
    }

    pub async fn eras(&self) -> ApiResult<NetworkErasResponse> {
        self.inner.client.get("network/eras", None).await
    }
}
