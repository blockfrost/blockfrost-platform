use crate::client::Dolos;
use bf_common::types::ApiResult;
use blockfrost_openapi::models::genesis_content::GenesisContent;

pub struct DolosGenesis<'a> {
    pub(crate) inner: &'a Dolos,
}

impl Dolos {
    pub fn genesis(&self) -> DolosGenesis<'_> {
        DolosGenesis { inner: self }
    }
}

impl DolosGenesis<'_> {
    pub async fn get(&self) -> ApiResult<GenesisContent> {
        self.inner.client.get("genesis", None).await
    }
}
