use crate::types::{
    AccountAddresses, AccountDelegations, AccountRegistrations, AccountResponse, AccountRewards,
};
use async_trait::async_trait;
use common::{errors::BlockfrostError, pagination::Pagination, types::ApiResult};

#[async_trait]
pub trait AccountsApi: Send + Sync + 'static {
    async fn accounts_stake_address(&self, _stake_address: &str) -> ApiResult<AccountResponse> {
        Err(BlockfrostError::not_found())
    }

    async fn accounts_stake_address_rewards(
        &self,
        _stake_address: &str,
    ) -> ApiResult<AccountRewards> {
        Err(BlockfrostError::not_found())
    }

    async fn accounts_stake_address_addresses(
        &self,
        _stake_address: &str,
        _pagination: &Pagination,
    ) -> ApiResult<AccountAddresses> {
        Err(BlockfrostError::not_found())
    }

    async fn accounts_stake_address_delegations(
        &self,
        _stake_address: &str,
        _pagination: &Pagination,
    ) -> ApiResult<AccountDelegations> {
        Err(BlockfrostError::not_found())
    }

    async fn accounts_stake_address_registrations(
        &self,
        _stake_address: &str,
        _pagination: &Pagination,
    ) -> ApiResult<AccountRegistrations> {
        Err(BlockfrostError::not_found())
    }
}
