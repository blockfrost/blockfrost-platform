use crate::client::Dolos;
use api_provider::{api::epochs::EpochsApi, types::EpochsParamResponse};
use async_trait::async_trait;
use common::types::ApiResult;

#[async_trait]
impl EpochsApi for Dolos {
    async fn epoch_number_parameters(&self, number: &str) -> ApiResult<EpochsParamResponse> {
        let path = format!("epochs/{number}/parameters");

        self.client.get(&path, None).await
    }

    async fn epoch_latest_parameters(&self) -> ApiResult<EpochsParamResponse> {
        self.client.get("epochs/latest/parameters", None).await
    }
}
