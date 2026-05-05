use super::pool_manager::NodePoolManager;
use bf_common::errors::AppError;
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
    pub fn new(
        network_magic: u64,
        socket_path: String,
        max_pool_connections: usize,
    ) -> Result<Self, AppError> {
        let manager = NodePoolManager {
            network_magic,
            socket_path,
        };
        let pool_manager = deadpool::managed::Pool::builder(manager)
            .max_size(max_pool_connections)
            .build()
            .map_err(|err| AppError::Node(err.to_string()))?;

        Ok(Self { pool_manager })
    }

    /// Borrows a single [`super::connection::NodeClient`] connection from the pool.
    pub async fn get(&self) -> Result<Object<NodePoolManager>, AppError> {
        self.pool_manager
            .get()
            .await
            .map_err(|err| AppError::Node(format!("NodeConnPool: {err}")))
    }
}
