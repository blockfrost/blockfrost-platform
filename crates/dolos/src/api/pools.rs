use crate::client::Dolos;
use bf_api_provider::types::{PoolsDelegatorsResponse, PoolsListExtendedResponse};
use bf_common::{pagination::Pagination, types::ApiResult};

pub struct DolosPools<'a> {
    pub(crate) inner: &'a Dolos,
}

impl Dolos {
    pub fn pools(&self) -> DolosPools<'_> {
        DolosPools { inner: self }
    }
}

impl DolosPools<'_> {
    pub async fn extended(&self, pagination: &Pagination) -> ApiResult<PoolsListExtendedResponse> {
        self.inner
            .client
            .get("pools/extended", Some(pagination))
            .await
    }

    pub async fn delegators(
        &self,
        pool_id: &str,
        pagination: &Pagination,
    ) -> ApiResult<PoolsDelegatorsResponse> {
        let path = format!("pools/{pool_id}/delegators");

        self.inner.client.get(&path, Some(pagination)).await
    }
}
