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

    pub async fn run(&self) {
        if let Some(icebreakers_api) = &self.icebreakers_api {
            'load_balancers: loop {
                match icebreakers_api.register().await {
                    Ok(response) => {
                        let configs: Vec<_> =
                            response.load_balancers.into_iter().flatten().collect();

                        if configs.is_empty() {
                            warn!("IceBreakers: no WebSocket load balancers to connect to");
                            break 'load_balancers;
                        }

                        load_balancer::run_all(
                            configs,
                            self.app.clone(),
                            self.health_errors.clone(),
                            self.api_prefix.clone(),
                        )
                        .await;

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
                        *self.health_errors.lock().await = vec![err.into()];
                        sleep(delay).await;
                    },
                }
            }
        }
    }
}
