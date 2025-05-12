use crate::BlockfrostError;
use pallas_network::{facades::NodeClient as NodeClientFacade, miniprotocols::localstate};
use std::{boxed::Box, pin::Pin};
use tokio::time::{Duration, timeout};
use tracing::error;

/// Our wrapper around [`pallas_network::facades::NodeClient`]. If you only use
/// this, you won’t get any deadlocks, inconsistencies, etc.
pub struct NodeClient {
    /// Note: this is an [`Option`] *only* to satisfy the borrow checker. It’s
    /// *always* [`Some`]. See [`<super::pool_manager::NodePoolManager as
    /// deadpool::managed::Manager>>::recycle`] for an explanation.
    pub(in crate::node) client: Option<NodeClientFacade>,
    pub(in crate::node) connection_id: u64,
    pub(in crate::node) unrecoverable_error_happened: bool,
    pub(in crate::node) network_magic: u64,
}

impl NodeClient {
    /// We always have to release the [`localstate::GenericClient`], even on errors,
    /// otherwise `cardano-node` stalls. If you use this function, it’s handled for you.
    pub async fn with_statequery_timeout<A, F>(
        &mut self,
        action: F,
        duration: Duration,
    ) -> Result<A, BlockfrostError>
    where
        F: for<'a> FnOnce(
            &'a mut localstate::GenericClient,
        ) -> Pin<
            Box<dyn std::future::Future<Output = Result<A, BlockfrostError>> + 'a + Sync + Send>,
        >,
    {
        // Acquire the client
        let client = self.client.as_mut().unwrap().statequery();

        client.acquire(None).await.inspect_err(|e| {
            error!(
                "N2C[{}]: failed to acquire a statequery client: {:?}",
                self.connection_id, e
            )
        })?;

        // Run the action with a timeout
        let result = timeout(duration, action(client)).await.map_err(|_| {
            let msg = format!("Timeout after {} seconds", duration.as_secs());
            error!(
                "N2C[{}]: {} in with_statequery_timeout",
                self.connection_id, msg
            );
            BlockfrostError::timeout(msg)
        })?;

        // Always release the client, even if action fails
        client.send_release().await.inspect_err(|e| {
            error!(
                "N2C[{}]: failed to release a statequery client: {:?}",
                self.connection_id, e
            )
        })?;

        result
    }

    pub async fn with_statequery<A, F>(&mut self, action: F) -> Result<A, BlockfrostError>
    where
        F: for<'a> FnOnce(
            &'a mut localstate::GenericClient,
        ) -> Pin<
            Box<dyn std::future::Future<Output = Result<A, BlockfrostError>> + 'a + Sync + Send>,
        >,
    {
        // Default timeout is 10 minutes
        self.with_statequery_timeout(action, Duration::from_secs(600))
            .await
    }

    /// Pings the node, e.g. to see if the connection is still alive.
    pub async fn ping(&mut self) -> Result<(), BlockfrostError> {
        // FIXME: we should be able to use `miniprotocols::keepalive`
        // (cardano-cli does), but for some reason it’s not added to
        // `NodeClient`? Let’s try to acquire a local state client instead:

        self.with_statequery_timeout(|_| Box::pin(async { Ok(()) }), Duration::from_secs(30))
            .await
    }

    /// After you call this, the pool will never use this N2C connection again.
    /// The need to do this arises rarely in normal operation, but it happens.
    pub fn invalidate_connection(&mut self, why: &str) {
        error!(
            "N2C[{}]: connection marked as invalid: {}",
            self.connection_id, why
        );
        self.unrecoverable_error_happened = true;
    }
}
