use crate::server::state::AppState;
use axum::extract::State;
use blockfrost_openapi::models::epoch_param_content::EpochParamContent;
use common::types::ApiResult;

pub async fn route(State(state): State<AppState>) -> ApiResult<EpochParamContent> {
    state.api.dolos.epoch_latest_parameters().await
}
