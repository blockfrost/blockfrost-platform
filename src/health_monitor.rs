use crate::{BlockfrostError, NodePool, node::sync_progress::NodeInfo};
use std::sync::Arc;
use tokio::sync::{Notify, RwLock};
use tokio::time::{self, Duration};

#[derive(Clone)]
pub struct HealthStatus {
    pub healthy: bool,
    pub errors: Vec<BlockfrostError>,
    pub node_info: Option<NodeInfo>,
}

#[derive(Clone)]
pub struct HealthMonitor {
    inner: Arc<RwLock<HealthStatus>>,
}

impl HealthMonitor {
    pub async fn current_status(&self) -> HealthStatus {
        self.inner.read().await.clone()
    }
}

const CHAIN_STALE_IF_OLDER_THAN: std::time::Duration = std::time::Duration::from_secs(5 * 60);

pub async fn spawn(node: NodePool) -> HealthMonitor {
    // This initial state is never seen:
    let state_ = Arc::new(RwLock::new(HealthStatus {
        healthy: false,
        errors: vec![],
        node_info: None,
    }));
    let state = state_.clone();

    let state_update_ = Arc::new(Notify::new());
    let state_update = state_update_.clone();

    tokio::spawn(async move {
        let mut last_chain_advancement = std::time::Instant::now();
        let mut last_chain_block =
            "0000000000000000000000000000000000000000000000000000000000000000".to_string();

        loop {
            let node_info: Result<NodeInfo, BlockfrostError> = async {
                let mut node = node.get().await?;
                node.sync_progress().await
            }
            .await;

            if let Ok(node_info) = node_info.as_ref() {
                if last_chain_block != node_info.block {
                    last_chain_block = node_info.block.clone();
                    last_chain_advancement = std::time::Instant::now();
                }
            }

            let chain_too_stale = {
                let elapsed = last_chain_advancement.elapsed();
                if elapsed > CHAIN_STALE_IF_OLDER_THAN {
                    let err = format!(
                        "Chain stuck at {}, has not seen updates in {:?}.",
                        last_chain_block, elapsed
                    );
                    tracing::error!("{}", err);
                    Some(BlockfrostError::internal_server_error(err))
                } else {
                    None
                }
            };

            let errors: Vec<BlockfrostError> = node_info
                .clone()
                .err()
                .into_iter()
                .chain(chain_too_stale.into_iter())
                .collect();
            let healthy = errors.is_empty();
            let node_info = node_info.ok();

            *(state.write().await) = HealthStatus {
                errors,
                healthy,
                node_info,
            };

            state_update.notify_one();

            // Set delay based on health status
            let delay = Duration::from_secs(if healthy { 10 } else { 2 });
            time::sleep(delay).await;
        }
    });

    state_update_.notified().await;

    HealthMonitor { inner: state_ }
}
