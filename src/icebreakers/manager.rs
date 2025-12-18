use crate::icebreakers::api::IcebreakersAPI;
use crate::server::state::ApiPrefix;
use crate::{hydra, load_balancer};
use axum::Router;
use bf_common::errors::BlockfrostError;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc, watch};
use tracing::{error, info, warn};

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

    pub async fn run_once(&self) -> Result<String, BlockfrostError> {
        let response = self.icebreakers_api.register().await?;
        let configs: Vec<_> = response.load_balancers.into_iter().flatten().collect();

        if configs.is_empty() {
            warn!("IceBreakers: no WebSocket load balancers to connect to");
            return Ok("No load balancers available".to_string());
        }

        let config_count = configs.len();

        tokio::spawn(load_balancer::run_all(
            configs,
            self.app.clone(),
            self.health_errors.clone(),
            self.api_prefix.clone(),
            None,
        ));

        let health_errors = self.health_errors.lock().await;

        if health_errors.is_empty() {
            Ok(format!("Started {config_count} load balancer connections"))
        } else {
            Ok(format!("Load balancer errors: {:?}", *health_errors))
        }
    }

    /// Runs the registration process periodically in a single spawned task.
    pub async fn run(
        self,
        hydra_kex: (
            mpsc::Receiver<hydra::KeyExchangeRequest>,
            mpsc::Sender<hydra::KeyExchangeResponse>,
        ),
    ) {
        let (dest_watch_tx, dest_watch_rx) = watch::channel(None);
        tokio::spawn(forward_to_changing_dest(hydra_kex.0, dest_watch_rx));

        // For now, we’re passing a pair with changeable destination of
        // requests, as we run multiple load balancers to multiple gateways:
        let mutable_hydra_kex: (
            watch::Sender<Option<mpsc::Sender<hydra::KeyExchangeRequest>>>,
            mpsc::Sender<hydra::KeyExchangeResponse>,
        ) = (dest_watch_tx, hydra_kex.1);

        tokio::spawn(async move {
            'load_balancers: loop {
                match self.icebreakers_api.register().await {
                    Ok(response) => {
                        let configs: Vec<_> =
                            response.load_balancers.into_iter().flatten().collect();
                        if configs.is_empty() {
                            warn!("IceBreakers: no WebSocket load balancers to connect to");
                            // If there are no load balancers, only register once, nothing to monitor:
                            break 'load_balancers;
                        }

                        load_balancer::run_all(
                            configs,
                            self.app.clone(),
                            self.health_errors.clone(),
                            self.api_prefix.clone(),
                            Some(mutable_hydra_kex.clone()),
                        )
                        .await;

                        let delay = std::time::Duration::from_secs(1);
                        info!("IceBreakers: will re-register in {:?}", delay);
                        tokio::time::sleep(delay).await;
                    },
                    Err(err) => {
                        let delay = std::time::Duration::from_secs(10);
                        error!(
                            "IceBreakers registration failed: {}, will re-register in {:?}",
                            err, delay
                        );

                        *self.health_errors.lock().await = vec![err.into()];
                        tokio::time::sleep(delay).await;
                    },
                }
            }
        });
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
