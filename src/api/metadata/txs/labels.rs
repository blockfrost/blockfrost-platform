use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::MetadataLabelsResponse;
use axum::extract::State;

pub async fn route(State(state): State<AppState>) -> ApiResult<MetadataLabelsResponse> {
    let dolos = state.get_dolos()?;

    dolos.metadata().labels().await
}
