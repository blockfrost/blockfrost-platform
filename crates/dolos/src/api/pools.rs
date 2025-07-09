use crate::client::Dolos;
use blockfrost_openapi::models::{
    pool_delegators_inner::PoolDelegatorsInner, pool_list_extended_inner::PoolListExtendedInner,
};
use common::types::ApiResult;

impl Dolos {
    pub async fn pools_extended(&self) -> ApiResult<Vec<PoolListExtendedInner>> {
        self.client.get("pools/extended").await
    }

    pub async fn pools_pool_id_delegators(
        &self,
        pool_id: &str,
    ) -> ApiResult<Vec<PoolDelegatorsInner>> {
        let path = format!("pools/{pool_id}/delegators");

        self.client.get(&path).await
    }
}
