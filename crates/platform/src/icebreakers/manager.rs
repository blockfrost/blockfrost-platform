use crate::icebreakers::api::IcebreakersAPI;
use crate::load_balancer;
use crate::server::state::ApiPrefix;
use axum::Router;
use bf_common::errors::BlockfrostError;
use std::sync::Arc;
use tokio::sync::Mutex;
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
        ));

        let health_errors = self.health_errors.lock().await;

        if health_errors.is_empty() {
            Ok(format!("Started {config_count} load balancer connections"))
        } else {
            Ok(format!("Load balancer errors: {:?}", *health_errors))
        }
    }

    /// Runs the registration process periodically in a single spawned task.
    pub async fn run(self) {
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
