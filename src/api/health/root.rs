use crate::{api::ApiResult, health_monitor::HealthMonitor};
use axum::{Extension, Json};
use blockfrost_openapi::models::_health_get_200_response::HealthGet200Response;

pub async fn route(
    Extension(health_monitor): Extension<HealthMonitor>,
) -> ApiResult<HealthGet200Response> {
    let mut is_healthy = true;
    let node_status = health_monitor.current_status().await;

    if !node_status.healthy {
        is_healthy = false;
    }

    let result = HealthGet200Response { is_healthy };

    Ok(Json(result))
}
