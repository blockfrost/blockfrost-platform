pub mod api;

use crate::api::{accounts::AccountsApi, addresses::AddressesApi};
use async_trait::async_trait;

/// Unified API interface for all data providers (e.g. Dolos, Node, etc.).
///
/// This trait defines a common structure for service-specific implementations.
/// Each method represents a supported endpoint and can be selectively overridden.
/// If a method is not implemented, it defaults to returning a 404 via `BlockfrostError::not_found()`.
///
/// Serves as a general abstraction layer over all backend services.
#[async_trait]
pub trait ApiProvider: AccountsApi + AddressesApi + Send + Sync + 'static {}
