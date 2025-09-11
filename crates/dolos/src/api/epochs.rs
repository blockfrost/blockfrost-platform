use crate::client::Dolos;
use api_provider::types::EpochsParamResponse;
use common::types::ApiResult;

pub struct DolosEpochs<'a> {
    pub(crate) inner: &'a Dolos,
}

impl Dolos {
    pub fn epochs(&self) -> DolosEpochs<'_> {
        DolosEpochs { inner: self }
    }
}

impl DolosEpochs<'_> {
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
