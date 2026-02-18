use crate::api::root::DataNodeRootResponse;
use crate::client::DataNode;
use bf_common::errors::BlockfrostError;
use std::sync::Arc;
use tokio::sync::Mutex;

pub struct DataNodeMonitor {
    errors: Arc<Mutex<Vec<BlockfrostError>>>,
    data_node_info: Arc<Mutex<Option<DataNodeRootResponse>>>,
}

impl Default for DataNodeMonitor {
    fn default() -> Self {
        Self::new()
    }
}

impl DataNodeMonitor {
    pub fn new() -> Self {
        Self {
            errors: Arc::new(Mutex::new(vec![])),
            data_node_info: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn update(&self, data_node: &Option<DataNode>) {
        let Some(data_node) = data_node else {
            // not configured, nothing to monitor
            *(self.errors.lock().await) = vec![];
            *(self.data_node_info.lock().await) = None;
            return;
        };

        // fetch root info
        let root_result = data_node.root().await;
        let (data_node_info, root_errors) = match root_result {
            Ok(info) => (Some(info.0), vec![]),
            Err(err) => {
                tracing::error!("Data node root check failed: {err}");
                (
                    None,
                    vec![BlockfrostError::internal_server_error(format!(
                        "Data node unreachable: {err}"
                    ))],
                )
            },
        };
        *(self.data_node_info.lock().await) = data_node_info;

        if !root_errors.is_empty() {
            *(self.errors.lock().await) = root_errors;
            return;
        }

        // Fetch health
        let health_result = data_node.health().get().await;
        let errors = match health_result {
            Ok(health) => {
                if health.is_healthy {
                    vec![]
                } else {
                    vec![BlockfrostError::internal_server_error(
                        "Data node reports unhealthy status".to_string(),
                    )]
                }
            },
            Err(err) => {
                tracing::error!("Data node health check failed: {err}");
                vec![BlockfrostError::internal_server_error(format!(
                    "Data node unreachable: {err}"
                ))]
            },
        };

        *(self.errors.lock().await) = errors;
    }

    pub fn errors(&self) -> Arc<Mutex<Vec<BlockfrostError>>> {
        self.errors.clone()
    }

    pub fn data_node_info(&self) -> Arc<Mutex<Option<DataNodeRootResponse>>> {
        self.data_node_info.clone()
    }
}
