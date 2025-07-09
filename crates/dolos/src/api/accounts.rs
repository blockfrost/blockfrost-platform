use crate::client::Dolos;
use blockfrost_openapi::models::{
    account_content::AccountContent,
    account_delegation_content_inner::AccountDelegationContentInner,
    account_registration_content_inner::AccountRegistrationContentInner,
    account_reward_content_inner::AccountRewardContentInner,
};
use common::types::ApiResult;

impl Dolos {
    pub async fn accounts_stake_address(&self, stake_address: &str) -> ApiResult<AccountContent> {
        let path = format!("accounts/{stake_address}");

        self.client.get(&path).await
    }

    pub async fn accounts_stake_address_rewards(
        &self,
        stake_address: &str,
    ) -> ApiResult<Vec<AccountRewardContentInner>> {
        let path = format!("accounts/{stake_address}/rewards");

        self.client.get(&path).await
    }

    pub async fn accounts_stake_address_addresses(
        &self,
        stake_address: &str,
    ) -> ApiResult<Vec<AccountContent>> {
        let path = format!("accounts/{stake_address}/addresses");
        self.client.get(&path).await
    }

    pub async fn accounts_stake_address_delegations(
        &self,
        stake_address: &str,
    ) -> ApiResult<Vec<AccountDelegationContentInner>> {
        let path = format!("accounts/{stake_address}/delegations");
        self.client.get(&path).await
    }

    pub async fn accounts_stake_address_registrations(
        &self,
        stake_address: &str,
    ) -> ApiResult<Vec<AccountRegistrationContentInner>> {
        let path = format!("accounts/{stake_address}/registrations");
        self.client.get(&path).await
    }
}
