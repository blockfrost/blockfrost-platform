use blockfrost_openapi::models::{
    account_addresses_content_inner::AccountAddressesContentInner, account_content::AccountContent,
    account_delegation_content_inner::AccountDelegationContentInner,
    account_registration_content_inner::AccountRegistrationContentInner,
    account_reward_content_inner::AccountRewardContentInner, address_content::AddressContent,
    address_utxo_content_inner::AddressUtxoContentInner, asset::Asset, block_content::BlockContent,
    epoch_content::EpochContent, epoch_param_content::EpochParamContent,
};

// accounts
pub type AccountResponse = AccountContent;
pub type AccountRewards = Vec<AccountRewardContentInner>;
pub type AccountDelegations = Vec<AccountDelegationContentInner>;
pub type AccountAddresses = Vec<AccountAddressesContentInner>;
pub type AccountRegistrations = Vec<AccountRegistrationContentInner>;

// addresses
pub type AddressResponse = AddressContent;
pub type AddressUtxos = Vec<AddressUtxoContentInner>;

// assets
pub type AssetResponse = Asset;

// blocks
pub type BlockResponse = BlockContent;

// epochs
pub type EpochParamResponse = EpochParamContent;
pub type EpochResponse = EpochContent;
