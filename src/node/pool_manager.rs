use super::connection::NodeClient;
use crate::AppError;
use deadpool::managed::{Manager, Metrics, RecycleError, RecycleResult};
use metrics::{counter, gauge};
use pallas_network::facades::NodeClient as NodeClientFacade;
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

        match NodeClientFacade::connect(&self.socket_path, self.network_magic).await {
            Ok(connection) => {
                info!(
                    "N2C[{}]: connection successfully established with a node socket: {}",
                    connection_id, self.socket_path
                );
                gauge!("cardano_node_connections").increment(1);

                Ok(NodeClient {
                    client: Some(connection),
                    connection_id,
                    unrecoverable_error_happened: false,
                })
            },
            Err(err) => {
                counter!("cardano_node_connections_failed").increment(1);
                error!(
                    "N2C[{}]: failed to connect to a node socket: {}: {:?}",
                    connection_id,
                    self.socket_path,
                    err.to_string()
                );
                Err(AppError::Node(err.to_string()))
            },
        }
    }

    /// Pallas decided to make the
    /// [`pallas_network::facades::NodeClient::abort`] take ownership of `self`.
    /// That’s why we need our `NodeClient::client` to be an [`Option`], because
    /// in here we only get a mutable reference. If the connection is broken, we
    /// have to call [`pallas_network::facades::NodeClient::abort`], because it
    /// joins certain multiplexer threads. Otherwise, it’s a resource leak.
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
                    "N2C[{}]: connection no longer viable: {}, {}, {:?}",
                    node.connection_id, self.socket_path, err, metrics
                );

                // Take ownership of the `NodeClient` from Pallas
                // This is the only moment when `client` becomes `None`.
                // I should not be used again.
                let owned = node.client.take().unwrap();

                counter!("cardano_node_connections_failed").increment(1);
                gauge!("cardano_node_connections").decrement(1);

                // Now call `abort` to clean up their resources:
                owned.abort().await;

                // And scrap the connection from the pool:
                Err(RecycleError::Backend(err))
            },
        }
    }
}
