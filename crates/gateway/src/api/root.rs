use crate::config::Config;
use crate::health_monitor::HealthMonitor;
use axum::{Extension, Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;

#[derive(Serialize)]
pub struct Response {
    pub url: Option<url::Url>,
    pub version: String,
    pub healthy: bool,
    pub commit: &'static str,
    pub errors: Vec<String>,
}

pub async fn route(
    Extension(config): Extension<Config>,
    Extension(health_monitor): Extension<HealthMonitor>,
) -> impl IntoResponse {
    let status = health_monitor.current_status().await;

    let http_status = if status.healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = Response {
        url: config.server.url.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        commit: env!("GIT_REVISION"),
        healthy: status.healthy,
        errors: status.errors,
    };

    (http_status, Json(response))
}
