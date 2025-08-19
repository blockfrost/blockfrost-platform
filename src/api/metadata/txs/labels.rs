use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::MetadataLabelsResponse;
use axum::extract::State;

pub async fn route(State(state): State<AppState>) -> ApiResult<MetadataLabelsResponse> {
    state.dolos.metadata().labels().await
}
