use crate::client::Dolos;
use blockfrost_openapi::models::genesis_content::GenesisContent;
use common::types::ApiResult;

impl Dolos {
    pub async fn genesis(&self) -> ApiResult<GenesisContent> {
        self.client.get("genesis", None).await
    }
}
