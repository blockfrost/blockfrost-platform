use super::pool_manager::NodePoolManager;
use crate::{AppError, cli::Config, genesis::get_all_network_magics};
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
    pub async fn new(config: &Config) -> Result<Self, AppError> {
        let candidates = get_all_network_magics();

        for &candidate in &candidates {
            let manager = NodePoolManager {
                network_magic: candidate,
                socket_path: config.node_socket_path.to_string(),
            };

            let pool_manager = deadpool::managed::Pool::builder(manager)
                .max_size(config.max_pool_connections)
                .build()
                .map_err(|err| AppError::Node(err.to_string()))?;

            let pool = Self { pool_manager };

            if pool.get().await.is_ok() {
                return Ok(pool);
            }
        }

        Err(AppError::Node(format!(
            "Unable to establish a connection using any of the network_magic values: {:?}",
            candidates
        )))
    }

    /// Borrows a single [`super::connection::NodeClient`] connection from the pool.
    pub async fn get(&self) -> Result<Object<NodePoolManager>, AppError> {
        self.pool_manager
            .get()
            .await
            .map_err(|err| AppError::Node(format!("NodeConnPool: {}", err)))
    }
}
