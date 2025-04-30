use crate::api::ApiResult;

use axum::Json;
use blockfrost_openapi::models::_health_clock_get_200_response::HealthClockGet200Response;
use chrono::Utc;

pub async fn route() -> ApiResult<HealthClockGet200Response> {
    let server_time = Utc::now().timestamp();

    Ok(Json(HealthClockGet200Response { server_time }))
}
