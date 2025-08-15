use crate::api::ApiResult;
use api_provider::types::HealthClockResponse;
use axum::Json;
use chrono::Utc;

pub async fn route() -> ApiResult<HealthClockResponse> {
    let server_time = Utc::now().timestamp();

    Ok(Json(HealthClockResponse { server_time }))
}
