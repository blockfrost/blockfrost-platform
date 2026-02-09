use crate::client::DataNode;
use bf_api_provider::types::DrepsSingleResponse;
use bf_common::types::ApiResult;

pub struct DataNodeGovernance<'a> {
    pub(crate) inner: &'a DataNode,
}

impl DataNode {
    pub fn governance(&self) -> DataNodeGovernance<'_> {
        DataNodeGovernance { inner: self }
    }
}

impl DataNodeGovernance<'_> {
    pub async fn drep(&self, drep_id: &str) -> ApiResult<DrepsSingleResponse> {
        let path = format!("governance/dreps/{drep_id}");
        self.inner.client.get(&path, None).await
    }
}
