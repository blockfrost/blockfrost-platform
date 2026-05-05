use crate::icebreakers::api::IcebreakersAPI;
use crate::server::state::ApiPrefix;
use crate::{hydra_client, load_balancer};
use axum::Router;
use bf_common::errors::BlockfrostError;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc, watch};

pub struct IcebreakersManager {
    icebreakers_api: Arc<IcebreakersAPI>,
    health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
    app: Router,
    api_prefix: ApiPrefix,
}

impl IcebreakersManager {
    pub fn new(
        icebreakers_api: Arc<IcebreakersAPI>,
        health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
        app: Router,
        api_prefix: ApiPrefix,
    ) -> Self {
        Self {
            icebreakers_api,
            health_errors,
            app,
            api_prefix,
        }
    }

    /// Spawns the load-balancer supervisor in a background task.
    ///
    /// The supervisor handles initial registration (with retries), connection
    /// management, and periodic re-registration to detect gateway list changes.
    pub async fn run(
        self,
        hydra_kex: (
            mpsc::Receiver<hydra_client::KeyExchangeRequest>,
            mpsc::Sender<hydra_client::KeyExchangeResponse>,
            mpsc::Sender<hydra_client::TerminateRequest>,
        ),
    ) {
        let (dest_watch_tx, dest_watch_rx) = watch::channel(None);
        tokio::spawn(forward_to_changing_dest(hydra_kex.0, dest_watch_rx));

        // For now, we’re passing a pair with changeable destination of
        // requests, as we run multiple load balancers to multiple gateways:
        let mutable_hydra_kex: (
            watch::Sender<Option<mpsc::Sender<hydra_client::KeyExchangeRequest>>>,
            mpsc::Sender<hydra_client::KeyExchangeResponse>,
            mpsc::Sender<hydra_client::TerminateRequest>,
        ) = (dest_watch_tx, hydra_kex.1, hydra_kex.2);

        tokio::spawn(load_balancer::run_all(
            self.app,
            self.health_errors,
            self.api_prefix,
            Some(mutable_hydra_kex),
            self.icebreakers_api,
        ));
    }
}

/// This helper forwards messages from `src` to a changing `dest_watch` channel.
///
/// You can also temporarily set the destination to `None` and no messages will
/// be lost in the meantime.
pub async fn forward_to_changing_dest<A: Send + 'static>(
    mut src: mpsc::Receiver<A>,
    mut dest_watch: watch::Receiver<Option<mpsc::Sender<A>>>,
) {
    while let Some(mut msg) = src.recv().await {
        // A `loop` to keep trying to deliver this `msg` until we either succeed
        // or know there will never be another destination:
        loop {
            let maybe_dest = dest_watch.borrow().clone();

            if let Some(dest) = maybe_dest {
                match dest.send(msg).await {
                    Ok(()) => {
                        // Delivered, move on to next message:
                        break;
                    },
                    Err(e) => {
                        // Destination channel closed, recover the value:
                        msg = e.0;

                        // Wait for destination to change:
                        if dest_watch.changed().await.is_err() {
                            // If no future destination → drop `msg` and end.
                            return;
                        }
                        // Then loop and try with the new destination.
                    },
                }
            } else {
                // No destination set yet; wait for one:
                if dest_watch.changed().await.is_err() {
                    // If no future destination → drop `msg` and end.
                    return;
                }
            }
        }
    }
}
