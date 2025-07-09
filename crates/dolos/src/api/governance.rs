use crate::client::Dolos;
use blockfrost_openapi::models::drep::Drep;
use common::types::ApiResult;

impl Dolos {
    pub async fn governance_dreps_drep_id(&self, drep_id: &str) -> ApiResult<Drep> {
        let path = &format!("governance/dreps/{drep_id}");

        self.client.get(path).await
    }
}
