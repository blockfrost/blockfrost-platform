use crate::client::Dolos;
use api_provider::types::{
    AccountsAddressesResponse, AccountsDelegationsResponse, AccountsRegistrationsResponse,
    AccountsResponse, AccountsRewardsResponse,
};
use common::{pagination::Pagination, types::ApiResult};

pub struct DolosAccounts<'a> {
    pub(crate) inner: &'a Dolos,
}

impl Dolos {
    pub fn accounts(&self) -> DolosAccounts<'_> {
        DolosAccounts { inner: self }
    }
}

impl DolosAccounts<'_> {
    pub async fn stake_address(&self, stake_address: &str) -> ApiResult<AccountsResponse> {
        let path = format!("accounts/{stake_address}");

        self.inner.client.get(&path, None).await
    }

    pub async fn rewards(&self, stake_address: &str) -> ApiResult<AccountsRewardsResponse> {
        let path = format!("accounts/{stake_address}/rewards");

        self.inner.client.get(&path, None).await
    }

    pub async fn addresses(
        &self,
        stake_address: &str,
        pagination: &Pagination,
    ) -> ApiResult<AccountsAddressesResponse> {
        let path = format!("accounts/{stake_address}/addresses");

        self.inner.client.get(&path, Some(pagination)).await
    }

    pub async fn delegations(
        &self,
        stake_address: &str,
        pagination: &Pagination,
    ) -> ApiResult<AccountsDelegationsResponse> {
        let path = format!("accounts/{stake_address}/delegations");

        self.inner.client.get(&path, Some(pagination)).await
    }

    pub async fn registrations(
        &self,
        stake_address: &str,
        pagination: &Pagination,
    ) -> ApiResult<AccountsRegistrationsResponse> {
        let path = format!("accounts/{stake_address}/registrations");

        self.inner.client.get(&path, Some(pagination)).await
    }
}
