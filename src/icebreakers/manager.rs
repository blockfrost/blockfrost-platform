use crate::icebreakers::api::IcebreakersAPI;
use crate::load_balancer;
use crate::server::state::ApiPrefix;
use axum::Router;
use common::errors::{AppError, BlockfrostError};
use std::{sync::Arc, time::Duration};
use tokio::sync::Mutex;
use tracing::{error, info, warn};

pub struct IcebreakersManager {
    icebreakers_api: Arc<IcebreakersAPI>,
    health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
    app: Router,
    api_prefix: ApiPrefix,
}

#[derive(Clone, Copy, Debug)]
pub enum RunMode {
    Once {
        detach: bool,
    },
    Periodic {
        ok_delay_secs: u64,
        err_delay_secs: u64,
    },
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

    pub async fn run(self, mode: RunMode) -> Result<Option<String>, AppError> {
        match mode {
            RunMode::Once { detach } => {
                let msg = self.tick(detach).await?;

                Ok(Some(msg))
            },
            RunMode::Periodic {
                ok_delay_secs,
                err_delay_secs,
            } => {
                tokio::spawn(async move {
                    loop {
                        match self.tick(false).await {
                            Ok(_msg) => {
                                let delay = Duration::from_secs(ok_delay_secs);
                                info!("IceBreakers: will re-register in {:?}", delay);

                                tokio::time::sleep(delay).await;
                            },
                            Err(err) => {
                                let delay = Duration::from_secs(err_delay_secs);

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

                Ok(None)
            },
        }
    }

    async fn tick(&self, detach: bool) -> Result<String, AppError> {
        let response = self.icebreakers_api.register().await?;
        let configs: Vec<_> = response.load_balancers.into_iter().flatten().collect();

        if configs.is_empty() {
            warn!("IceBreakers: no WebSocket load balancers to connect to");
            return Ok("No load balancers available".to_string());
        }

        let config_count = configs.len();

        if detach {
            tokio::spawn(load_balancer::run_all(
                configs,
                self.app.clone(),
                self.health_errors.clone(),
                self.api_prefix.clone(),
            ));
        } else {
            load_balancer::run_all(
                configs,
                self.app.clone(),
                self.health_errors.clone(),
                self.api_prefix.clone(),
            )
            .await;
        }

        let health_errors = self.health_errors.lock().await;

        if health_errors.is_empty() {
            Ok(format!("Started {config_count} load balancer connections"))
        } else {
            Ok(format!("Load balancer errors: {:?}", *health_errors))
        }
    }
}
