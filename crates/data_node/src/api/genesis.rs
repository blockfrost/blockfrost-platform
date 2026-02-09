use crate::client::DataNode;
use bf_api_provider::types::GenesisResponse;
use bf_common::types::ApiResult;

pub struct DataNodeGenesis<'a> {
    pub(crate) inner: &'a DataNode,
}

impl DataNode {
    pub fn genesis(&self) -> DataNodeGenesis<'_> {
        DataNodeGenesis { inner: self }
    }
}

impl DataNodeGenesis<'_> {
    pub async fn get(&self) -> ApiResult<GenesisResponse> {
        self.inner.client.get("genesis", None).await
    }
}
