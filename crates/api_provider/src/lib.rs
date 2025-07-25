pub mod api;
pub mod types;

use crate::api::{
    accounts::AccountsApi, addresses::AddressesApi, assets::AssetsApi, blocks::BlocksApi,
    epochs::EpochsApi, genesis::GenesisApi, governance::GovernanceApi, metadata::MetadataApi,
    network::NetworkApi, pools::PoolsApi, txs::TxsApi,
};
use async_trait::async_trait;

/// Unified API interface for all data providers (e.g. Dolos, Node, etc.).
///
/// This trait defines a common structure for service-specific implementations.
/// Each method represents a supported endpoint and can be selectively overridden.
/// If a method is not implemented, it defaults to returning a 404 via `BlockfrostError::not_found()`.
///
/// Serves as a general abstraction layer over all backend services.
#[async_trait]
pub trait ApiProvider:
    AccountsApi
    + AddressesApi
    + AssetsApi
    + BlocksApi
    + EpochsApi
    + NetworkApi
    + GenesisApi
    + GovernanceApi
    + MetadataApi
    + TxsApi
    + PoolsApi
    + Send
    + Sync
    + 'static
{
}

#[async_trait]
impl<T> ApiProvider for T where
    T: AccountsApi
        + AddressesApi
        + AssetsApi
        + BlocksApi
        + EpochsApi
        + NetworkApi
        + GovernanceApi
        + MetadataApi
        + GenesisApi
        + PoolsApi
        + TxsApi
        + Send
        + Sync
        + 'static
{
}
