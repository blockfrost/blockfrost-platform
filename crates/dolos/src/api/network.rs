use crate::client::Dolos;
use api_provider::types::{NetworkErasResponse, NetworkResponse};
use common::types::ApiResult;

pub struct DolosNetwork<'a> {
    pub(crate) inner: &'a Dolos,
}

impl Dolos {
    pub fn network(&self) -> DolosNetwork<'_> {
        DolosNetwork { inner: self }
    }
}

impl DolosNetwork<'_> {
    pub async fn get(&self) -> ApiResult<NetworkResponse> {
        self.inner.client.get("network", None).await
    }

    pub async fn eras(&self) -> ApiResult<NetworkErasResponse> {
        self.inner.client.get("network/eras", None).await
    }
}
