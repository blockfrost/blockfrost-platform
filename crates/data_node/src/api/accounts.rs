use crate::client::DataNode;
use bf_api_provider::types::{
    AccountsAddressesResponse, AccountsDelegationsResponse, AccountsRegistrationsResponse,
    AccountsResponse, AccountsRewardsResponse,
};
use bf_common::{pagination::Pagination, types::ApiResult};

pub struct DataNodeAccounts<'a> {
    pub(crate) inner: &'a DataNode,
}

impl DataNode {
    pub fn accounts(&self) -> DataNodeAccounts<'_> {
        DataNodeAccounts { inner: self }
    }
}

impl DataNodeAccounts<'_> {
    pub async fn stake_address(&self, stake_address: &str) -> ApiResult<AccountsResponse> {
        let path = format!("accounts/{stake_address}");

        self.inner.client.get(&path, None).await
    }

    pub async fn rewards(
        &self,
        stake_address: &str,
        pagination: &Pagination,
    ) -> ApiResult<AccountsRewardsResponse> {
        let path = format!("accounts/{stake_address}/rewards");

        self.inner.client.get(&path, Some(pagination)).await
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
