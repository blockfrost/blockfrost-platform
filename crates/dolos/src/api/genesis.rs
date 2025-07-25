use crate::client::Dolos;
use api_provider::api::genesis::GenesisApi;
use async_trait::async_trait;
use blockfrost_openapi::models::genesis_content::GenesisContent;
use common::types::ApiResult;

#[async_trait]
impl GenesisApi for Dolos {
    async fn genesis(&self) -> ApiResult<GenesisContent> {
        self.client.get("genesis", None).await
    }
}
