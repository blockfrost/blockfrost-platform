use crate::{health_monitor::HealthMonitor, node::sync_progress::NodeInfo};
use axum::{Extension, Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RootResponse {
    pub name: String,
    pub version: String,
    pub revision: String,
    pub healthy: bool,
    pub node_info: Option<NodeInfo>,
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
        healthy: status.healthy,
        errors: status.errors.into_iter().map(|e| e.to_string()).collect(),
    };

    (http_status, Json(response))
}
