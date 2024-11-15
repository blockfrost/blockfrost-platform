use super::connection::NodeClient;
use crate::errors::AppError;
use deadpool::managed::{Manager, Metrics, RecycleError, RecycleResult};
use metrics::gauge;
use pallas_network::facades::NodeClient as NodeClientFacade;
use pallas_network::facades::{self};
use tokio_retry::{strategy::ExponentialBackoff, RetryIf};
use tracing::{error, info};

pub struct NodePoolManager {
    pub network_magic: u64,
    pub socket_path: String,
}

impl Manager for NodePoolManager {
    type Type = NodeClient;
    type Error = AppError;

    async fn create(&self) -> Result<NodeClient, AppError> {
        let retry_strategy = ExponentialBackoff::from_millis(100)
            .factor(2)
            .max_delay(std::time::Duration::from_secs(5))
            .take(5);

        let socket_path = self.socket_path.clone();
        let network_magic = self.network_magic;

        let attempt_connect = || async {
            match NodeClientFacade::connect(&socket_path, network_magic).await {
                Ok(conn) => Ok(conn),
                Err(err) => {
                    error!(
                        "Failed to connect to N2C node socket: {}: {:?}",
                        socket_path, err
                    );
                    Err(err)
                }
            }
        };

        // Retry on all errors
        let should_retry = |err: &facades::Error| {
            error!("Retrying N2C connection: {:?}", err);
            true
        };

        // Attempt to connect with retries
        match RetryIf::spawn(retry_strategy, attempt_connect, should_retry).await {
            Ok(conn) => {
                info!(
                    "N2C connection to node successfully established at socket: {}",
                    self.socket_path
                );

                gauge!("cardano_node_connections").increment(1);

                Ok(NodeClient { client: Some(conn) })
            }
            Err(err) => {
                error!(
                    "Failed to establish N2C connection after retries: {}: {:?}",
                    self.socket_path, err
                );
                Err(AppError::Node(err.to_string()))
            }
        }
    }

    /// Pallas decided to make the
    /// [`pallas_network::facades::NodeClient::abort`] take ownership of `self`.
    /// That’s why we need our [`Node::client`] to be an [`Option`],
    /// because in here we only get a mutable reference. If the connection is
    /// broken, we have to call `abort`, because it joins certain multiplexer
    /// threads. Otherwise, it’s a resource leak.
    async fn recycle(&self, node: &mut NodeClient, metrics: &Metrics) -> RecycleResult<AppError> {
        // Check if the connection is still viable
        match node.ping().await {
            Ok(_) => Ok(()),
            Err(err) => {
                error!(
                    "N2C connection no longer viable: {}, {}, {:?}",
                    self.socket_path, err, metrics
                );

                // Take ownership of the `NodeClient` from Pallas
                // This is the only moment when `client` becomes `None`.
                // I should not be used again.
                let owned = node.client.take().unwrap();

                // Now call `abort` to clean up their resources:
                owned.abort().await;

                // And scrap the connection from the pool:
                Err(RecycleError::Backend(AppError::Node(err.to_string())))
            }
        }
    }
}
