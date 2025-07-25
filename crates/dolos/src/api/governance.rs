use crate::client::Dolos;
use api_provider::api::governance::GovernanceApi;
use async_trait::async_trait;
use blockfrost_openapi::models::drep::Drep;
use common::types::ApiResult;

#[async_trait]
impl GovernanceApi for Dolos {
    async fn governance_dreps_drep_id(&self, drep_id: &str) -> ApiResult<Drep> {
        let path = format!("governance/dreps/{drep_id}");

        self.client.get(&path, None).await
    }
}
