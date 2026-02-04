use crate::health_monitor::HealthMonitor;
use axum::{Extension, Json, http::StatusCode, response::IntoResponse};
use bf_data_node::api::root::DataNodeRootResponse;
use bf_node::sync_progress::NodeInfo;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RootResponse {
    pub name: String,
    pub version: String,
    pub revision: String,
    pub healthy: bool,
    pub node_info: Option<NodeInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data_node: Option<DataNodeRootResponse>,
    pub errors: Vec<String>,
}

pub async fn route(Extension(health_monitor): Extension<HealthMonitor>) -> impl IntoResponse {
    let status = health_monitor.current_status().await;

    let http_status = if status.healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = RootResponse {
        name: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        revision: env!("GIT_REVISION").to_string(),
        node_info: status.node_info,
        data_node: status.data_node_info,
        healthy: status.healthy,
        errors: status.errors.into_iter().map(|e| e.to_string()).collect(),
    };

    (http_status, Json(response))
}
