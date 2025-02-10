use crate::BlockfrostError;
use pallas_network::{facades::NodeClient as NodeClientFacade, miniprotocols::localstate};
use std::{boxed::Box, pin::Pin};
use tracing::warn;

/// Our wrapper around [`pallas_network::facades::NodeClient`]. If you only use
/// this, you won’t get any deadlocks, inconsistencies, etc.
pub struct NodeClient {
    /// Note: this is an [`Option`] *only* to satisfy the borrow checker. It’s
    /// *always* [`Some`]. See [`<super::pool_manager::NodePoolManager as
    /// deadpool::managed::Manager>>::recycle`] for an explanation.
    pub(in crate::node) client: Option<NodeClientFacade>,
}

impl NodeClient {
    /// We always have to release the [`localstate::GenericClient`], even on errors,
    /// otherwise `cardano-node` stalls. If you use this function, it’s handled for you.
    pub async fn with_statequery<A, F>(&mut self, action: F) -> Result<A, BlockfrostError>
    where
        F: for<'a> FnOnce(
            &'a mut localstate::GenericClient,
        ) -> Pin<
            Box<dyn std::future::Future<Output = Result<A, BlockfrostError>> + 'a + Sync + Send>,
        >,
    {
        // Acquire the client
        let client = self.client.as_mut().unwrap().statequery();
        client.acquire(None).await?;

        // Run the action and ensure the client is released afterwards
        let result = action(client).await;

        // Always release the client, even if action fails
        if let Err(e) = client.send_release().await {
            warn!("Failed to release client: {:?}", e);
        }

        result
    }

    /// Pings the node, e.g. to see if the connection is still alive.
    pub async fn ping(&mut self) -> Result<(), BlockfrostError> {
        // FIXME: we should be able to use `miniprotocols::keepalive`
        // (cardano-cli does), but for some reason it’s not added to
        // `NodeClient`? Let’s try to acquire a local state client instead:

        self.with_statequery(|_| Box::pin(async { Ok(()) })).await
    }
}
