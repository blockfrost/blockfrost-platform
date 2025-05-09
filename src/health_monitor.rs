use crate::{BlockfrostError, NodePool, node::sync_progress::NodeInfo};
use std::sync::Arc;
use tokio::sync::{Mutex, Notify};
use tokio::time::{self, Duration};

#[derive(Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub errors: Vec<BlockfrostError>,
    pub node_info: Option<NodeInfo>,
}

/// An alias for Clippy:
type ErrorSource = Arc<Mutex<Vec<BlockfrostError>>>;

#[derive(Clone)]
pub struct HealthMonitor {
    sources: Arc<Mutex<Vec<ErrorSource>>>,
    node_info: Arc<Mutex<Option<NodeInfo>>>,
}

impl HealthMonitor {
    /// This is what `GET /` calls.
    pub async fn current_status(&self) -> HealthStatus {
        let errors = Self::collect_errors(&self.sources.lock().await).await;
        let node_info = self.node_info.lock().await.clone();
        let healthy = errors.is_empty();
        HealthStatus {
            errors,
            node_info,
            healthy,
        }
    }

    /// Gets the number of currently happening errors for Prometheus metrics.
    pub async fn num_errors(&self) -> u32 {
        Self::collect_errors(&self.sources.lock().await).await.len() as u32
    }

    /// Collect errors across multiple sources.
    async fn collect_errors(sources: &[Arc<Mutex<Vec<BlockfrostError>>>]) -> Vec<BlockfrostError> {
        let mut errors = vec![];
        for src in sources {
            let errs = src.lock().await;
            for error in errs.iter() {
                errors.push(error.clone());
            }
        }
        errors
    }

    /// Adds this shared vector as one of the sources of errors reported under
    /// `GET /`. You can then modify the vectors (including emptying them) to
    /// modify the final reported list. A way to compose more persistent errors
    /// from different parts of the app.
    pub async fn register_error_source(&self, src: Arc<Mutex<Vec<BlockfrostError>>>) {
        self.sources.lock().await.push(src)
    }

    /// Starts various health monitors in the background.
    pub async fn spawn(node: NodePool) -> Self {
        let node_mon = node_monitor::NodeMonitor::new();
        let mut chain_mon = chain_staleness_monitor::ChainStalenessMonitor::new();

        let self_ = Self {
            sources: Arc::new(Mutex::new(vec![])),
            node_info: node_mon.node_info(),
        };

        self_.register_error_source(node_mon.errors()).await;
        self_.register_error_source(chain_mon.errors()).await;

        let notify_state_update = Arc::new(Notify::new());
        let notify_state_update_ = notify_state_update.clone();

        tokio::spawn(async move {
            let mut previously_healthy = true;
            loop {
                node_mon.update(&node).await;
                chain_mon
                    .update(&*(node_mon.node_info().lock().await))
                    .await;
                notify_state_update_.notify_one();

                // Set delay based on health status
                let node_healthy = Self::collect_errors(&[node_mon.errors(), chain_mon.errors()])
                    .await
                    .is_empty();

                if previously_healthy && !node_healthy {
                    tracing::warn!("Node pool became unhealthy.");
                } else if !previously_healthy && node_healthy {
                    tracing::info!("Node pool became healthy again.");
                }
                previously_healthy = node_healthy;

                let delay = Duration::from_secs(if node_healthy { 10 } else { 2 });

                time::sleep(delay).await;
            }
        });

        notify_state_update.notified().await;
        self_
    }
}

mod node_monitor {
    use crate::{BlockfrostError, NodePool, node::sync_progress::NodeInfo};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    pub struct NodeMonitor {
        errors: Arc<Mutex<Vec<BlockfrostError>>>,
        node_info: Arc<Mutex<Option<NodeInfo>>>,
    }

    impl NodeMonitor {
        pub fn new() -> Self {
            Self {
                errors: Arc::new(Mutex::new(vec![])),
                node_info: Arc::new(Mutex::new(None)),
            }
        }

        pub async fn update(&self, node: &NodePool) {
            let node_info: Result<NodeInfo, BlockfrostError> = async {
                let mut node = node.get().await?;
                node.sync_progress().await
            }
            .await;

            let (node_info, errors) = match node_info {
                Ok(a) => (Some(a), vec![]),
                Err(err) => (None, vec![err]),
            };

            *(self.errors.lock().await) = errors;
            *(self.node_info.lock().await) = node_info;
        }

        pub fn errors(&self) -> Arc<Mutex<Vec<BlockfrostError>>> {
            self.errors.clone()
        }

        pub fn node_info(&self) -> Arc<Mutex<Option<NodeInfo>>> {
            self.node_info.clone()
        }
    }
}

mod chain_staleness_monitor {
    use crate::{BlockfrostError, node::sync_progress::NodeInfo};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    const CHAIN_STALE_IF_OLDER_THAN: std::time::Duration = std::time::Duration::from_secs(5 * 60);

    pub struct ChainStalenessMonitor {
        last_chain_advancement: std::time::Instant,
        last_chain_block: String,
        errors: Arc<Mutex<Vec<BlockfrostError>>>,
    }

    impl ChainStalenessMonitor {
        pub fn new() -> Self {
            Self {
                last_chain_advancement: std::time::Instant::now(),
                last_chain_block:
                    "0000000000000000000000000000000000000000000000000000000000000000".to_string(),
                errors: Arc::new(Mutex::new(vec![])),
            }
        }

        pub async fn update(&mut self, node_info: &Option<NodeInfo>) {
            if let Some(node_info) = node_info {
                if self.last_chain_block != node_info.block {
                    self.last_chain_block = node_info.block.clone();
                    self.last_chain_advancement = std::time::Instant::now();
                }
            }

            let elapsed = self.last_chain_advancement.elapsed();

            *(self.errors.lock().await) = if elapsed > CHAIN_STALE_IF_OLDER_THAN {
                let err = format!(
                    "Chain stuck at {}, has not seen updates in {:?}.",
                    self.last_chain_block, elapsed
                );
                tracing::error!("{}", err);
                vec![BlockfrostError::internal_server_error(err)]
            } else {
                vec![]
            };
        }

        pub fn errors(&self) -> Arc<Mutex<Vec<BlockfrostError>>> {
            self.errors.clone()
        }
    }
}
