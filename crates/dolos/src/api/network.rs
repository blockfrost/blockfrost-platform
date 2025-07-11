use crate::client::Dolos;
use blockfrost_openapi::models::{network::Network, network_eras_inner::NetworkErasInner};
use common::types::ApiResult;

impl Dolos {
    pub async fn network(&self) -> ApiResult<Network> {
        self.client.get("network").await
    }

    pub async fn network_eras(&self) -> ApiResult<Vec<NetworkErasInner>> {
        self.client.get("network/eras").await
    }
}
