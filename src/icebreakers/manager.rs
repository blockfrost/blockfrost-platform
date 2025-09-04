use crate::icebreakers::api::IcebreakersAPI;
use crate::load_balancer;
use crate::server::state::ApiPrefix;
use axum::Router;
use common::errors::BlockfrostError;
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::time::{Duration, sleep};
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

    /// Executes a single registration attempt and runs load balancers if successful.
    /// Returns a string describing the registration outcome.
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

    /// Runs the registration process periodically
    pub async fn run(self) {
        loop {
            match self.run_once().await {
                Ok(response) => {
                    info!("IceBreakers: registration outcome: {}", response);

                    let delay = Duration::from_secs(1);
                    info!("IceBreakers: will re-register in {:?}", delay);
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
