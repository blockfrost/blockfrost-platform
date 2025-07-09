use crate::client::Dolos;
use blockfrost_openapi::models::epoch_param_content::EpochParamContent;
use common::types::ApiResult;

impl Dolos {
    pub async fn epoch_number_parameters(&self, number: &str) -> ApiResult<EpochParamContent> {
        let path = format!("epochs/{number}/parameters");

        self.client.get(&path).await
    }

    pub async fn epoch_latest_parameters(&self) -> ApiResult<EpochParamContent> {
        self.client.get("epochs/latest/parameters").await
    }
}
