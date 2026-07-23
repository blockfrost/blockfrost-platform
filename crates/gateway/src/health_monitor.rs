use crate::blockfrost::BlockfrostAPI;
use crate::db::DB;
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tokio::time::{self, Duration, Instant};

#[derive(Clone)]
pub struct HealthError {
    /// Stable machine-readable code, safe to expose in `GET /`.
    pub code: &'static str,
    /// Detailed reason; only logged server-side, never exposed over HTTP.
    pub detail: String,
}

#[derive(Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub errors: Vec<HealthError>,
}

impl HealthStatus {
    pub fn details(&self) -> String {
        self.errors
            .iter()
            .map(|e| e.detail.as_str())
            .collect::<Vec<_>>()
            .join("; ")
    }
}

#[derive(Clone)]
pub struct HealthMonitor {
    status: Arc<Mutex<HealthStatus>>,
}

impl HealthMonitor {
    const DB_PING_TIMEOUT: Duration = Duration::from_secs(5);
    const BLOCKFROST_PING_TIMEOUT: Duration = Duration::from_secs(5);
    /// The Blockfrost check runs on its own, slower cadence to keep the
    /// request-quota usage of the health checks negligible.
    const BLOCKFROST_CHECK_INTERVAL: Duration = Duration::from_secs(60);
    const HEALTHY_CHECK_INTERVAL: Duration = Duration::from_secs(10);
    const UNHEALTHY_CHECK_INTERVAL: Duration = Duration::from_secs(2);

    pub async fn current_status(&self) -> HealthStatus {
        self.status.lock().await.clone()
    }

    pub fn new_static(status: HealthStatus) -> Self {
        Self {
            status: Arc::new(Mutex::new(status)),
        }
    }

    pub async fn spawn(db: DB, blockfrost_api: BlockfrostAPI) -> Self {
        let self_ = Self::new_static(HealthStatus {
            healthy: true,
            errors: vec![],
        });
        let status = self_.status.clone();

        let first_check_done = Arc::new(Notify::new());
        let first_check_done_ = first_check_done.clone();

        tokio::spawn(async move {
            // `None` until the first check completes, so that we only log
            // actual changes (startup failures are handled by `main`).
            let mut previous_codes: Option<Vec<&'static str>> = None;
            let mut blockfrost_error: Option<String> = None;
            let mut last_blockfrost_check: Option<Instant> = None;
            loop {
                let db_error = db.ping(Self::DB_PING_TIMEOUT).await.err();

                if last_blockfrost_check
                    .is_none_or(|at| at.elapsed() >= Self::BLOCKFROST_CHECK_INTERVAL)
                {
                    blockfrost_error = blockfrost_api
                        .ping(Self::BLOCKFROST_PING_TIMEOUT)
                        .await
                        .err();
                    last_blockfrost_check = Some(Instant::now());
                }

                let errors: Vec<HealthError> = [
                    db_error.map(|detail| HealthError {
                        code: "database_unreachable",
                        detail,
                    }),
                    blockfrost_error.clone().map(|detail| HealthError {
                        code: "blockfrost_api_unreachable",
                        detail,
                    }),
                ]
                .into_iter()
                .flatten()
                .collect();
                let healthy = errors.is_empty();

                let codes: Vec<&'static str> = errors.iter().map(|e| e.code).collect();
                let status_ = HealthStatus { healthy, errors };

                if previous_codes.is_some() && previous_codes.as_ref() != Some(&codes) {
                    if healthy {
                        tracing::warn!("Gateway became healthy again.");
                    } else {
                        tracing::warn!("Gateway unhealthy: {}", status_.details());
                    }
                }

                previous_codes = Some(codes);

                *status.lock().await = status_;

                first_check_done_.notify_one();

                let delay = if healthy {
                    Self::HEALTHY_CHECK_INTERVAL
                } else {
                    Self::UNHEALTHY_CHECK_INTERVAL
                };

                time::sleep(delay).await;
            }
        });

        first_check_done.notified().await;
        self_
    }
}
