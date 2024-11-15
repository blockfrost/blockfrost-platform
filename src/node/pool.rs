use super::pool_manager::NodePoolManager;
use crate::{cli::Config, errors::AppError};
use deadpool::managed::{Object, Pool};

/// This represents a pool of Node2Client connections to a single `cardano-node`.
///
/// It can be safely cloned to multiple threads, while still sharing the same
/// set of underlying connections to the node.
#[derive(Clone)]
pub struct NodePool {
    pool_manager: Pool<NodePoolManager>,
}

impl NodePool {
    /// Creates a new pool of [`NodeConn`] connections.
    pub fn new(config: &Config) -> Result<Self, AppError> {
        let manager = NodePoolManager {
            network_magic: config.network_magic,
            socket_path: config.node_socket_path.to_string(),
        };
        let pool_manager = deadpool::managed::Pool::builder(manager)
            .max_size(config.max_pool_connections)
            .build()
            .map_err(|err| AppError::Node(err.to_string()))?;

        Ok(Self { pool_manager })
    }

    /// Borrows a single [`NodeConn`] connection from the pool.
    pub async fn get(&self) -> Result<Object<NodePoolManager>, AppError> {
        self.pool_manager
            .get()
            .await
            .map_err(|err| AppError::Node(format!("NodeConnPool: {}", err)))
    }
}
