use blockfrost_openapi::models::{
    _health_clock_get_200_response::HealthClockGet200Response,
    _health_get_200_response::HealthGet200Response,
    account_addresses_assets_inner::AccountAddressesAssetsInner,
    account_addresses_content_inner::AccountAddressesContentInner,
    account_addresses_total::AccountAddressesTotal, account_content::AccountContent,
    account_delegation_content_inner::AccountDelegationContentInner,
    account_history_content_inner::AccountHistoryContentInner,
    account_mir_content_inner::AccountMirContentInner,
    account_registration_content_inner::AccountRegistrationContentInner,
    account_reward_content_inner::AccountRewardContentInner,
    account_utxo_content_inner::AccountUtxoContentInner,
    account_withdrawal_content_inner::AccountWithdrawalContentInner,
    address_content::AddressContent, address_content_extended::AddressContentExtended,
    address_content_total::AddressContentTotal,
    address_transactions_content_inner::AddressTransactionsContentInner,
    address_utxo_content_inner::AddressUtxoContentInner, asset::Asset,
    asset_addresses_inner::AssetAddressesInner, asset_history_inner::AssetHistoryInner,
    asset_policy_inner::AssetPolicyInner, asset_transactions_inner::AssetTransactionsInner,
    assets_inner::AssetsInner, block_content::BlockContent,
    block_content_addresses_inner::BlockContentAddressesInner, drep::Drep,
    drep_delegators_inner::DrepDelegatorsInner, drep_metadata::DrepMetadata,
    drep_updates_inner::DrepUpdatesInner, drep_votes_inner::DrepVotesInner,
    epoch_content::EpochContent, epoch_param_content::EpochParamContent,
    epoch_stake_content_inner::EpochStakeContentInner,
    epoch_stake_pool_content_inner::EpochStakePoolContentInner, genesis_content::GenesisContent,
    network::Network, network_eras_inner::NetworkErasInner, pool::Pool,
    pool_delegators_inner::PoolDelegatorsInner, pool_history_inner::PoolHistoryInner,
    pool_list_extended_inner::PoolListExtendedInner, pool_list_retire_inner::PoolListRetireInner,
    pool_metadata::PoolMetadata, pool_updates_inner::PoolUpdatesInner, proposal::Proposal,
    proposal_parameters_parameters::ProposalParametersParameters,
    proposal_votes_inner::ProposalVotesInner, proposal_withdrawals_inner::ProposalWithdrawalsInner,
    proposals_inner::ProposalsInner, script::Script, script_cbor::ScriptCbor,
    script_datum_cbor::ScriptDatumCbor, script_json::ScriptJson,
    script_redeemers_inner::ScriptRedeemersInner, scripts_inner::ScriptsInner,
    tx_content::TxContent, tx_content_cbor::TxContentCbor,
    tx_content_delegations_inner::TxContentDelegationsInner,
    tx_content_metadata_cbor_inner::TxContentMetadataCborInner,
    tx_content_metadata_inner::TxContentMetadataInner, tx_content_mirs_inner::TxContentMirsInner,
    tx_content_pool_certs_inner::TxContentPoolCertsInner,
    tx_content_pool_certs_inner_relays_inner::TxContentPoolCertsInnerRelaysInner,
    tx_content_pool_retires_inner::TxContentPoolRetiresInner,
    tx_content_redeemers_inner::TxContentRedeemersInner,
    tx_content_required_signers_inner::TxContentRequiredSignersInner,
    tx_content_stake_addr_inner::TxContentStakeAddrInner, tx_content_utxo::TxContentUtxo,
    tx_content_withdrawals_inner::TxContentWithdrawalsInner,
    tx_metadata_label_cbor_inner::TxMetadataLabelCborInner,
    tx_metadata_label_json_inner::TxMetadataLabelJsonInner,
    tx_metadata_labels_inner::TxMetadataLabelsInner,
};

// health
pub type HealthClockResponse = HealthClockGet200Response;
pub type HealthResponse = HealthGet200Response;

// accounts
pub type AccountsResponse = AccountContent;
pub type AccountsAddressesTotalResponse = AccountAddressesTotal;
pub type AccountsRewardsResponse = Vec<AccountRewardContentInner>;
pub type AccountsDelegationsResponse = Vec<AccountDelegationContentInner>;
pub type AccountsAddressesResponse = Vec<AccountAddressesContentInner>;
pub type AccountsAssetsResponse = Vec<AccountAddressesAssetsInner>;
pub type AccountsRegistrationsResponse = Vec<AccountRegistrationContentInner>;
pub type AccountsHistoryResponse = Vec<AccountHistoryContentInner>;
pub type AccountsMirResponse = Vec<AccountMirContentInner>;
pub type AccountsUtxosResponse = Vec<AccountUtxoContentInner>;
pub type AccountsWithdrawalsResponse = Vec<AccountWithdrawalContentInner>;

// addresses
pub type AddressesResponse = AddressContent;
pub type AddressesContentExtendedResponse = AddressContentExtended;
pub type AddressesUtxosResponse = Vec<AddressUtxoContentInner>;
pub type AddressesContentTotalResponse = AddressContentTotal;
pub type AddressesTransactionsResponse = Vec<AddressTransactionsContentInner>;

// assets
pub type AssetsSingleResponse = Asset;
pub type AssetsResponse = Vec<AssetsInner>;
pub type AssetsPolicyResponse = Vec<AssetPolicyInner>;
pub type AssetsTransactionsResponse = Vec<AssetTransactionsInner>;
pub type AssetsHistoryResponse = Vec<AssetHistoryInner>;
pub type AssetsAddressesResponse = Vec<AssetAddressesInner>;

// blocks
pub type BlocksSingleResponse = BlockContent;
pub type BlocksResponse = Vec<BlockContent>;
pub type BlocksAddressesExtendedResponse = Vec<AddressContentExtended>;
pub type BlocksAddressesContentResponse = BlockContentAddressesInner;

// epochs
pub type EpochsParamResponse = EpochParamContent;
pub type EpochsResponse = EpochContent;
pub type EpochsStakeResponse = Vec<EpochStakeContentInner>;
pub type EpochStakePoolResponse = Vec<EpochStakePoolContentInner>;

// networks
pub type NetworkResponse = Network;
pub type NetworkErasResponse = Vec<NetworkErasInner>;

// governance
pub type DrepsSingleResponse = Drep;
pub type DrepsDelegatorsResponse = Vec<DrepDelegatorsInner>;
pub type DrepsMetadataResponse = DrepMetadata;
pub type DrepsUpdatesResponse = Vec<DrepUpdatesInner>;
pub type DrepsVotesResponse = Vec<DrepVotesInner>;
pub type DrepsProposalsResponse = Vec<ProposalsInner>;
pub type DrepsProposalParametersResponse = ProposalParametersParameters;
pub type DrepsProposalVotesResponse = Vec<ProposalVotesInner>;
pub type DrepsSingleProposalResponse = Proposal;
pub type DrepsProposalWithdrawalsResponse = Vec<ProposalWithdrawalsInner>;

// metadata
pub type MetadataLabelsResponse = Vec<TxMetadataLabelsInner>;
pub type MetadataLabelJsonResponse = Vec<TxMetadataLabelJsonInner>;
pub type MetadataLabelCborResponse = Vec<TxMetadataLabelCborInner>;

// scripts
pub type ScriptsSingleResponse = Script;
pub type ScriptsDatumCborResponse = Vec<ScriptDatumCbor>;
pub type ScriptsInnerResponse = Vec<ScriptsInner>;
pub type ScriptsCborResponse = Vec<ScriptCbor>;
pub type ScriptsJsonResponse = Vec<ScriptJson>;
pub type ScriptsRedeemersInnerResponse = Vec<ScriptRedeemersInner>;

// pools
pub type PoolsSingleResponse = Pool;
pub type PoolsListExtendedResponse = Vec<PoolListExtendedInner>;
pub type PoolsDelegatorsResponse = Vec<PoolDelegatorsInner>;
pub type PoolsRetiresResponse = Vec<PoolListRetireInner>;
pub type PoolsHistoryResponse = Vec<PoolHistoryInner>;
pub type PoolsMetadataResponse = PoolMetadata;
pub type PoolsUpdatesResponse = Vec<PoolUpdatesInner>;
pub type PoolsProposalVotesResponse = Vec<ProposalVotesInner>;

// txs
pub type TxsSingleResponse = TxContent;
pub type TxsCborResponse = TxContentCbor;
pub type TxsUtxosResponse = TxContentUtxo;
pub type TxsMetadataResponse = Vec<TxContentMetadataInner>;
pub type TxsMetadataCborResponse = Vec<TxContentMetadataCborInner>;
pub type TxsWithdrawalsResponse = Vec<TxContentWithdrawalsInner>;
pub type TxsDelegationsResponse = Vec<TxContentDelegationsInner>;
pub type TxsMirsResponse = Vec<TxContentMirsInner>;
pub type TxsPoolRetiresResponse = Vec<TxContentPoolRetiresInner>;
pub type TxsStakeAddrResponse = Vec<TxContentStakeAddrInner>;
pub type TxsPoolCertsResponse = Vec<TxContentPoolCertsInner>;
pub type TxsPoolCertsInnerRelaysResponse = Vec<TxContentPoolCertsInnerRelaysInner>;
pub type TxsRedeemersResponse = Vec<TxContentRedeemersInner>;
pub type TxsContentRequiredSignersInner = Vec<TxContentRequiredSignersInner>;

// genesis
pub type GenesisResponse = GenesisContent;

// tests
pub type TestsAddressUtxoResponse = AddressUtxoContentInner;
