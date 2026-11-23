use super::connection::NodeClient;
use bf_common::errors::AppError;
use deadpool::managed::{Manager, Metrics, RecycleError, RecycleResult};
use metrics::{counter, gauge};
use std::sync::atomic;
use tracing::{error, info};

pub struct NodePoolManager {
    pub network_magic: u64,
    pub socket_path: String,
}

static N2C_CONNECTION_COUNTER: atomic::AtomicU64 = atomic::AtomicU64::new(0);

impl Manager for NodePoolManager {
    type Type = NodeClient;
    type Error = AppError;

    async fn create(&self) -> Result<NodeClient, AppError> {
        // TODO: maybe use `ExponentialBackoff` from `tokio-retry`, to have at
        // least _some_ debouncing between requests, if the node is down?
        counter!("cardano_node_connections_initiated").increment(1);
        let connection_id = 1 + N2C_CONNECTION_COUNTER.fetch_add(1, atomic::Ordering::SeqCst);

        match NodeClient::connect(&self.socket_path, self.network_magic, connection_id).await {
            Ok(connection) => {
                info!(
                    connection_id,
                    "connection successfully established with a node socket: {}", self.socket_path
                );
                gauge!("cardano_node_connections").increment(1);

                Ok(connection)
            },
            Err(err) => {
                counter!("cardano_node_connections_failed").increment(1);
                error!(
                    connection_id,
                    "failed to connect to a node socket: {}: {}", self.socket_path, err
                );
                Err(AppError::Node(err.to_string()))
            },
        }
    }

    async fn recycle(&self, node: &mut NodeClient, metrics: &Metrics) -> RecycleResult<AppError> {
        let can_communicate = if node.unrecoverable_error_happened {
            Err(AppError::Node(
                "unrecoverable error happened previously".to_string(),
            ))
        } else {
            node.ping()
                .await
                .map_err(|err| AppError::Node(err.to_string()))
        };

        // Check if the connection is still viable
        match can_communicate {
            Ok(_) => Ok(()),
            Err(err) => {
                error!(
                    connection_id = node.connection_id,
                    "connection no longer viable: {}, {}, {:?}", self.socket_path, err, metrics
                );

                counter!("cardano_node_connections_failed").increment(1);
                node.close().await;

                // And scrap the connection from the pool:
                Err(RecycleError::Backend(err))
            },
        }
    }

    fn detach(&self, _node: &mut NodeClient) {
        gauge!("cardano_node_connections").decrement(1);
    }
}
