use blockfrost_openapi::models::{
    account_addresses_content_inner::AccountAddressesContentInner, account_content::AccountContent,
    account_delegation_content_inner::AccountDelegationContentInner,
    account_registration_content_inner::AccountRegistrationContentInner,
    account_reward_content_inner::AccountRewardContentInner, address_content::AddressContent,
    address_utxo_content_inner::AddressUtxoContentInner,
};

pub type AccountResponse = AccountContent;
pub type AccountRewards = Vec<AccountRewardContentInner>;
pub type AccountDelegations = Vec<AccountDelegationContentInner>;
pub type AccountAddresses = Vec<AccountAddressesContentInner>;
pub type AccountRegistrations = Vec<AccountRegistrationContentInner>;

pub type AddressResponse = AddressContent;
pub type AddressUtxos = Vec<AddressUtxoContentInner>;
