use super::connection::NodeClient;
use crate::{cbor::fallback_decoder::FallbackDecoder, AppError};
use deadpool::managed::{Manager, Metrics, RecycleError, RecycleResult};
use metrics::gauge;
use pallas_network::facades::NodeClient as NodeClientFacade;
use tracing::{error, info};

pub struct NodePoolManager {
    pub network_magic: u64,
    pub socket_path: String,
    pub fallback_decoder: FallbackDecoder,
}

impl Manager for NodePoolManager {
    type Type = NodeClient;
    type Error = AppError;

    async fn create(&self) -> Result<NodeClient, AppError> {
        // TODO: maybe use `ExponentialBackoff` from `tokio-retry`, to have at
        // least _some_ debouncing between requests, if the node is down?
        match NodeClientFacade::connect(&self.socket_path, self.network_magic).await {
            Ok(connection) => {
                info!(
                    "N2C connection to node was successfully established at socket: {}",
                    self.socket_path
                );
                gauge!("cardano_node_connections").increment(1);

                Ok(NodeClient {
                    client: Some(connection),
                    fallback_decoder: self.fallback_decoder.clone(),
                })
            }
            Err(err) => {
                error!(
                    "Failed to connect a node socket: {}: {:?}",
                    self.socket_path,
                    err.to_string()
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
