use crate::client::Dolos;
use bf_api_provider::types::DrepsSingleResponse;
use bf_common::types::ApiResult;

pub struct DolosGovernance<'a> {
    pub(crate) inner: &'a Dolos,
}

impl Dolos {
    pub fn governance(&self) -> DolosGovernance<'_> {
        DolosGovernance { inner: self }
    }
}

impl DolosGovernance<'_> {
    pub async fn drep(&self, drep_id: &str) -> ApiResult<DrepsSingleResponse> {
        let path = format!("governance/dreps/{drep_id}");
        self.inner.client.get(&path, None).await
    }
}
