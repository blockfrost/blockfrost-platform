use super::pool_manager::NodePoolManager;
use crate::{
    AppError,
    config::Config,
    genesis::{GenesisRegistry, genesis},
};
use deadpool::managed::{Object, Pool};

/// This represents a pool of `NodeToClient` connections to a single `cardano-node`.
///
/// It can be safely cloned to multiple threads, while still sharing the same
/// set of underlying connections to the node.
#[derive(Clone)]
pub struct NodePool {
    pool_manager: Pool<NodePoolManager>,
}

impl NodePool {
    /// Creates a new pool of [`super::connection::NodeClient`] connections.
    pub fn new(config: &Config) -> Result<Self, AppError> {
        let network_magic = genesis().by_network(&config.network).network_magic as u64;

        let manager = NodePoolManager {
            network_magic,
            socket_path: config.node_socket_path.to_string(),
        };
        let pool_manager = deadpool::managed::Pool::builder(manager)
            .max_size(config.max_pool_connections)
            .build()
            .map_err(|err| AppError::Node(err.to_string()))?;

        Ok(Self { pool_manager })
    }

    /// Borrows a single [`super::connection::NodeClient`] connection from the pool.
    pub async fn get(&self) -> Result<Object<NodePoolManager>, AppError> {
        self.pool_manager
            .get()
            .await
            .map_err(|err| AppError::Node(format!("NodeConnPool: {}", err)))
    }
}
