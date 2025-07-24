use crate::client::Dolos;
use api_provider::api::{
    accounts::AccountsApi,
    types::{
        AccountAddresses, AccountDelegations, AccountRegistrations, AccountResponse, AccountRewards,
    },
};
use async_trait::async_trait;
use common::{pagination::Pagination, types::ApiResult};

#[async_trait]
impl AccountsApi for Dolos {
    async fn accounts_stake_address(&self, stake_address: &str) -> ApiResult<AccountResponse> {
        let path = format!("accounts/{stake_address}");

        self.client.get(&path, None).await
    }

    async fn accounts_stake_address_rewards(
        &self,
        stake_address: &str,
    ) -> ApiResult<AccountRewards> {
        let path = format!("accounts/{stake_address}/rewards");

        self.client.get(&path, None).await
    }

    async fn accounts_stake_address_addresses(
        &self,
        stake_address: &str,
        pagination: &Pagination,
    ) -> ApiResult<AccountAddresses> {
        let path = format!("accounts/{stake_address}/addresses");

        self.client.get(&path, Some(pagination)).await
    }

    async fn accounts_stake_address_delegations(
        &self,
        stake_address: &str,
        pagination: &Pagination,
    ) -> ApiResult<AccountDelegations> {
        let path = format!("accounts/{stake_address}/delegations");

        self.client.get(&path, Some(pagination)).await
    }

    async fn accounts_stake_address_registrations(
        &self,
        stake_address: &str,
        pagination: &Pagination,
    ) -> ApiResult<AccountRegistrations> {
        let path = format!("accounts/{stake_address}/registrations");

        self.client.get(&path, Some(pagination)).await
    }
}
