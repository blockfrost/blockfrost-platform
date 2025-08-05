use crate::client::Dolos;
use api_provider::{
    api::network::NetworkApi,
    types::{NetworkErasResponse, NetworkResponse},
};
use async_trait::async_trait;
use common::types::ApiResult;

#[async_trait]
impl NetworkApi for Dolos {
    async fn network(&self) -> ApiResult<NetworkResponse> {
        self.client.get("network", None).await
    }

    async fn network_eras(&self) -> ApiResult<NetworkErasResponse> {
        self.client.get("network/eras", None).await
    }
}
