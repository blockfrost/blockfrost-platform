use crate::types::DrepsSingleResponse;
use async_trait::async_trait;
use common::{errors::BlockfrostError, types::ApiResult};

#[async_trait]
pub trait GovernanceApi: Send + Sync + 'static {
    async fn governance_dreps_drep_id(&self, _drep_id: &str) -> ApiResult<DrepsSingleResponse> {
        Err(BlockfrostError::not_found())
    }
}
