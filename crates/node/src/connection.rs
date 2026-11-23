use bf_common::errors::BlockfrostError;
use pallas_network::{
    facades::NodeClient as NodeClientFacade,
    miniprotocols::{handshake, localstate},
    multiplexer::Bearer,
};
use std::{boxed::Box, pin::Pin};
use tokio::time::{Duration, timeout};
use tracing::error;

/// Our wrapper around [`pallas_network::facades::NodeClient`]. If you only use
/// this, you won’t get any deadlocks, inconsistencies, etc.
pub struct NodeClient {
    /// Taken when the connection is closed after an interrupted exchange.
    pub(crate) client: Option<NodeClientFacade>,
    pub(crate) connection_id: u64,
    pub(crate) unrecoverable_error_happened: bool,
    pub(crate) network_magic: u64,
    pub(crate) cleanup_runtime: tokio::runtime::Handle,
}

impl NodeClient {
    /// Connect and handshake while retaining ownership for cancellation cleanup.
    pub(crate) async fn connect(
        socket_path: &str,
        network_magic: u64,
        connection_id: u64,
    ) -> Result<Self, bf_common::errors::AppError> {
        use bf_common::errors::AppError;

        #[cfg(unix)]
        let bearer = Bearer::connect_unix(socket_path)
            .await
            .map_err(|err| AppError::Node(err.to_string()))?;
        #[cfg(windows)]
        let bearer = {
            let path = socket_path.to_owned();
            tokio::task::spawn_blocking(move || Bearer::connect_named_pipe(path))
                .await
                .map_err(|err| AppError::Node(err.to_string()))?
                .map_err(|err| AppError::Node(err.to_string()))?
        };

        let mut node = Self {
            client: Some(NodeClientFacade::new(bearer)),
            connection_id,
            unrecoverable_error_happened: false,
            network_magic,
            cleanup_runtime: tokio::runtime::Handle::current(),
        };
        let confirmation = node
            .client
            .as_mut()
            .unwrap()
            .handshake()
            .handshake(handshake::n2c::VersionTable::v10_and_above(network_magic))
            .await
            .map_err(|err| AppError::Node(err.to_string()))?;
        if let handshake::Confirmation::Rejected(reason) = confirmation {
            return Err(AppError::Node(format!(
                "Node handshake rejected: {reason:?}"
            )));
        }
        Ok(node)
    }

    /// Bound acquisition, the action and release by one deadline.
    /// An interrupted exchange must be closed instead of sending Release while
    /// the peer may still have agency.
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
        if self.unrecoverable_error_happened {
            return Err(BlockfrostError::internal_server_error(
                "Node connection is no longer usable".to_string(),
            ));
        }

        // Leave this set if the caller cancels the future mid-exchange.
        self.unrecoverable_error_happened = true;
        let client = self.client.as_mut().unwrap().statequery();
        let result = timeout(duration, async {
            client.acquire(None).await?;
            let result = action(client).await;
            if *client.state() == localstate::State::Acquired {
                client.send_release().await?;
                Ok(result)
            } else {
                Err(BlockfrostError::internal_server_error(
                    "State query ended before the node returned agency".to_string(),
                ))
            }
        })
        .await;

        match result {
            Ok(Ok(result)) => {
                self.unrecoverable_error_happened = false;
                result
            },
            Ok(Err(err)) => {
                self.close().await;
                Err(err)
            },
            Err(_) => {
                let msg = format!("Timeout after {} seconds", duration.as_secs());
                self.invalidate_connection(&msg);
                self.close().await;
                Err(BlockfrostError::timeout(msg))
            },
        }
    }

    pub(crate) async fn close(&mut self) {
        self.unrecoverable_error_happened = true;
        if let Some(client) = self.client.take() {
            client.abort().await;
        }
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
            connection_id = self.connection_id,
            "connection marked as invalid: {}", why
        );
        self.unrecoverable_error_happened = true;
    }
}

// Pool creation and recycling deadlines can drop an exchange before it returns.
// Dropping Pallas itself only detaches its multiplexer tasks, so abort explicitly.
impl Drop for NodeClient {
    fn drop(&mut self) {
        if let Some(client) = self.client.take() {
            self.cleanup_runtime.spawn(client.abort());
        }
    }
}
