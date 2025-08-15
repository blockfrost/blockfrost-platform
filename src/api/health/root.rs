use crate::{api::ApiResult, health_monitor::HealthMonitor};
use api_provider::types::HealthResponse;
use axum::{Extension, Json};

pub async fn route(
    Extension(health_monitor): Extension<HealthMonitor>,
) -> ApiResult<HealthResponse> {
    let mut is_healthy = true;
    let node_status = health_monitor.current_status().await;

    if !node_status.healthy {
        is_healthy = false;
    }

    let result = HealthResponse { is_healthy };

    Ok(Json(result))
}
