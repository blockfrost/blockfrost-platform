use crate::{pool::NodePool, sync_progress::NodeInfo};
use bf_common::errors::BlockfrostError;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct NodeMonitor {
    errors: Arc<Mutex<Vec<BlockfrostError>>>,
    node_info: Arc<Mutex<Option<NodeInfo>>>,
}

impl Default for NodeMonitor {
    fn default() -> Self {
        Self::new()
    }
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
