use crate::types::{
    AccountsAddressesResponse, AccountsDelegationsResponse, AccountsRegistrationsResponse,
    AccountsResponse, AccountsRewardsResponse,
};
use async_trait::async_trait;
use common::{errors::BlockfrostError, pagination::Pagination, types::ApiResult};

#[async_trait]
pub trait AccountsApi: Send + Sync + 'static {
    async fn accounts_stake_address(&self, _stake_address: &str) -> ApiResult<AccountsResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn accounts_stake_address_rewards(
        &self,
        _stake_address: &str,
    ) -> ApiResult<AccountsRewardsResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn accounts_stake_address_addresses(
        &self,
        _stake_address: &str,
        _pagination: &Pagination,
    ) -> ApiResult<AccountsAddressesResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn accounts_stake_address_delegations(
        &self,
        _stake_address: &str,
        _pagination: &Pagination,
    ) -> ApiResult<AccountsDelegationsResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn accounts_stake_address_registrations(
        &self,
        _stake_address: &str,
        _pagination: &Pagination,
    ) -> ApiResult<AccountsRegistrationsResponse> {
        Err(BlockfrostError::not_found())
    }
}
