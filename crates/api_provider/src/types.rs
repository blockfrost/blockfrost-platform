use blockfrost_openapi::models::{
    account_addresses_content_inner::AccountAddressesContentInner, account_content::AccountContent,
    account_delegation_content_inner::AccountDelegationContentInner,
    account_registration_content_inner::AccountRegistrationContentInner,
    account_reward_content_inner::AccountRewardContentInner, address_content::AddressContent,
    address_utxo_content_inner::AddressUtxoContentInner, asset::Asset, block_content::BlockContent,
    drep::Drep, epoch_content::EpochContent, epoch_param_content::EpochParamContent,
    genesis_content::GenesisContent, network::Network, network_eras_inner::NetworkErasInner,
    pool_delegators_inner::PoolDelegatorsInner, pool_list_extended_inner::PoolListExtendedInner,
    tx_content::TxContent, tx_content_cbor::TxContentCbor,
    tx_content_delegations_inner::TxContentDelegationsInner,
    tx_content_metadata_cbor_inner::TxContentMetadataCborInner,
    tx_content_metadata_inner::TxContentMetadataInner, tx_content_mirs_inner::TxContentMirsInner,
    tx_content_pool_certs_inner::TxContentPoolCertsInner,
    tx_content_pool_retires_inner::TxContentPoolRetiresInner,
    tx_content_stake_addr_inner::TxContentStakeAddrInner, tx_content_utxo::TxContentUtxo,
    tx_content_withdrawals_inner::TxContentWithdrawalsInner,
    tx_metadata_label_cbor_inner::TxMetadataLabelCborInner,
    tx_metadata_label_json_inner::TxMetadataLabelJsonInner,
    tx_metadata_labels_inner::TxMetadataLabelsInner,
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

// networks
pub type NetworkResponse = Network;
pub type NetworkErasResponse = Vec<NetworkErasInner>;

// governance
pub type DrepResposne = Drep;

// metadata
pub type MetadataLabelsResponse = Vec<TxMetadataLabelsInner>;
pub type MetadataLabelJsonResponse = Vec<TxMetadataLabelJsonInner>;
pub type MetadataLabelCborResponse = Vec<TxMetadataLabelCborInner>;

// pools
pub type PoolListExtendedResponse = Vec<PoolListExtendedInner>;
pub type PoolDelegatorsResponse = Vec<PoolDelegatorsInner>;

// txs
pub type TxResponse = TxContent;
pub type TxCborResponse = TxContentCbor;
pub type TxUtxosResponse = TxContentUtxo;
pub type TxMetadataResponse = Vec<TxContentMetadataInner>;
pub type TxMetadataCborResponse = Vec<TxContentMetadataCborInner>;
pub type TxWithdrawalsResponse = Vec<TxContentWithdrawalsInner>;
pub type TxDelegationsResponse = Vec<TxContentDelegationsInner>;
pub type TxMirsResponse = Vec<TxContentMirsInner>;
pub type TxPoolRetiresResponse = Vec<TxContentPoolRetiresInner>;
pub type TxStakeAddrResponse = Vec<TxContentStakeAddrInner>;
pub type TxPoolCertsResponse = Vec<TxContentPoolCertsInner>;

// genesis
pub type GenesisResponse = GenesisContent;
