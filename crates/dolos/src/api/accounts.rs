use crate::client::Dolos;
use blockfrost_openapi::models::{
    account_content::AccountContent,
    account_delegation_content_inner::AccountDelegationContentInner,
    account_registration_content_inner::AccountRegistrationContentInner,
    account_reward_content_inner::AccountRewardContentInner,
};
use common::{pagination::Pagination, types::ApiResult};

impl Dolos {
    pub async fn accounts_stake_address(&self, stake_address: &str) -> ApiResult<AccountContent> {
        let path = format!("accounts/{stake_address}");

        self.client.get(&path, None).await
    }

    pub async fn accounts_stake_address_rewards(
        &self,
        stake_address: &str,
    ) -> ApiResult<Vec<AccountRewardContentInner>> {
        let path = format!("accounts/{stake_address}/rewards");

        self.client.get(&path, None).await
    }

    pub async fn accounts_stake_address_addresses(
        &self,
        stake_address: &str,
        pagination: &Pagination,
    ) -> ApiResult<Vec<AccountContent>> {
        let path = format!("accounts/{stake_address}/addresses");

        self.client.get(&path, Some(pagination)).await
    }

    pub async fn accounts_stake_address_delegations(
        &self,
        stake_address: &str,
        pagination: &Pagination,
    ) -> ApiResult<Vec<AccountDelegationContentInner>> {
        let path = format!("accounts/{stake_address}/delegations");

        self.client.get(&path, Some(pagination)).await
    }

    pub async fn accounts_stake_address_registrations(
        &self,
        stake_address: &str,
        pagination: &Pagination,
    ) -> ApiResult<Vec<AccountRegistrationContentInner>> {
        let path = format!("accounts/{stake_address}/registrations");

        self.client.get(&path, Some(pagination)).await
    }
}
