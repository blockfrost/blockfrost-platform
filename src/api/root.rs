use crate::{health_monitor::HealthMonitor, server::state::AppState};
use axum::{Extension, Json, extract::State, http::StatusCode};
use node::sync_progress::NodeInfo;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RootResponse {
    pub name: String,
    pub version: String,
    pub revision: String,
    pub healthy: bool,
    pub node_info: Option<NodeInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub data_nodes: Vec<DolosInfo>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DolosInfo {
    pub name: String,
    pub version: String,
    pub revision: String,
}

pub async fn route(
    Extension(health_monitor): Extension<HealthMonitor>,
    State(state): State<AppState>,
) -> (StatusCode, Json<RootResponse>) {
    let status = health_monitor.current_status().await;
    let (base_healthy, node_info, mut errors): (bool, Option<NodeInfo>, Vec<String>) = (
        status.healthy,
        status.node_info,
        status.errors.into_iter().map(|e| e.to_string()).collect(),
    );

    let mut data_nodes: Vec<DolosInfo> = Vec::new();

    let mut dolos_healthy = true;

    if state.config.data_sources.dolos.is_some() {
        match state.get_dolos() {
            Ok(dolos) => match dolos.root().get().await {
                Ok(root_response) => {
                    data_nodes.push(DolosInfo {
                        name: "dolos".into(),
                        version: root_response.version.clone(),
                        revision: root_response.revision.clone(),
                    });
                },
                Err(e) => {
                    dolos_healthy = false;
                    errors.push(format!("dolos root fetching info error: {e}"));
                },
            },
            Err(e) => {
                dolos_healthy = false;
                errors.push(format!("dolos init error: {e}"));
            },
        }
    }

    let http_status = if base_healthy && dolos_healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let body = RootResponse {
        name: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        revision: env!("GIT_REVISION").to_string(),
        healthy: base_healthy,
        node_info,
        data_nodes,
        errors,
    };

    (http_status, Json(body))
}
