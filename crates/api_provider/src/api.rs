pub mod accounts;
pub mod addresses;
pub mod assets;
pub mod blocks;
pub mod epochs;
pub mod genesis;
pub mod governance;
pub mod metadata;
pub mod network;
pub mod pools;
pub mod txs;

use crate::ApiProvider;
use std::sync::Arc;

/// Central access point for all API providers (e.g. Dolos, Node, etc.).
///
/// This struct groups together different implementations of the `ApiProvider` trait,
/// allowing consumers to call unified methods on a specific backend service via:
/// `api.dolos.method(...)` or `api.node.method(...)`.
pub struct Api {
    pub dolos: Arc<dyn ApiProvider>,
    // pub node: Arc<dyn ApiProvider>,
}

impl Api {
    pub fn new(dolos: Arc<dyn ApiProvider>) -> Self {
        Self { dolos }
    }
}
