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
        loop {
            let node_info: Result<NodeInfo, BlockfrostError> = async {
                let mut node = node.get().await?;
                node.sync_progress().await
            }
            .await;
            let errors: Vec<BlockfrostError> = node_info.clone().err().into_iter().collect();
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
