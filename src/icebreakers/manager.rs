use crate::icebreakers::api::IcebreakersAPI;
use crate::load_balancer;
use crate::server::state::ApiPrefix;
use common::errors::BlockfrostError;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};
use tracing::{error, info, warn};

pub struct IcebreakersManager {
    icebreakers_api: Option<Arc<IcebreakersAPI>>,
    health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
    app: axum::Router,
    api_prefix: ApiPrefix,
}

impl IcebreakersManager {
    pub fn new(
        icebreakers_api: Option<Arc<IcebreakersAPI>>,
        health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
        app: axum::Router,
        api_prefix: ApiPrefix,
    ) -> Self {
        Self {
            icebreakers_api,
            health_errors,
            app,
            api_prefix,
        }
    }

    /// Executes a single registration attempt and runs load balancers if successful.
    /// Returns a string describing the registration outcome.
    pub async fn run_once(&self) -> Result<Option<String>, BlockfrostError> {
        if let Some(icebreakers_api) = &self.icebreakers_api {
            let response = icebreakers_api.register().await?;
            let configs: Vec<_> = response.load_balancers.into_iter().flatten().collect();

            if configs.is_empty() {
                warn!("IceBreakers: no WebSocket load balancers to connect to");
                return Ok(Some("No load balancers available".to_string()));
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
                Ok(Some(format!(
                    "Started {config_count} load balancer connections"
                )))
            } else {
                Ok(Some(format!("Load balancer errors: {:?}", *health_errors)))
            }
        } else {
            Ok(None)
        }
    }

    /// Runs the registration process periodically
    pub async fn run(self) {
        if self.icebreakers_api.is_none() {
            return;
        }

        loop {
            match self.run_once().await {
                Ok(Some(response)) => {
                    info!("IceBreakers: registration outcome: {}", response);

                    let delay = Duration::from_secs(1);
                    info!("IceBreakers: will re-register in {:?}", delay);
                    sleep(delay).await;
                },
                Ok(None) => {
                    info!("IceBreakers: no API available, skipping registration");

                    let delay = Duration::from_secs(1);
                    sleep(delay).await;
                },
                Err(err) => {
                    let delay = Duration::from_secs(10);

                    error!(
                        "IceBreakers registration failed: {}, will re-register in {:?}",
                        err, delay
                    );

                    *self.health_errors.lock().await = vec![err];
                    sleep(delay).await;
                },
            }
        }
    }
}
