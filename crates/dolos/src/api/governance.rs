use crate::client::Dolos;
use blockfrost_openapi::models::drep::Drep;
use common::types::ApiResult;

pub struct DolosGovernance<'a> {
    pub(crate) inner: &'a Dolos,
}

impl Dolos {
    pub fn governance(&self) -> DolosGovernance<'_> {
        DolosGovernance { inner: self }
    }
}

impl DolosGovernance<'_> {
    pub async fn drep(&self, drep_id: &str) -> ApiResult<Drep> {
        let path = format!("governance/dreps/{drep_id}");
        self.inner.client.get(&path, None).await
    }
}
