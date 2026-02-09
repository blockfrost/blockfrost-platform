use crate::client::DataNode;
use bf_api_provider::types::EpochsParamResponse;
use bf_common::types::ApiResult;

pub struct DataNodeEpochs<'a> {
    pub(crate) inner: &'a DataNode,
}

impl DataNode {
    pub fn epochs(&self) -> DataNodeEpochs<'_> {
        DataNodeEpochs { inner: self }
    }
}

impl DataNodeEpochs<'_> {
    pub async fn parameters(&self, number: &i32) -> ApiResult<EpochsParamResponse> {
        let path = format!("epochs/{number}/parameters");

        self.inner.client.get(&path, None).await
    }

    pub async fn latest_parameters(&self) -> ApiResult<EpochsParamResponse> {
        self.inner
            .client
            .get("epochs/latest/parameters", None)
            .await
    }
}
