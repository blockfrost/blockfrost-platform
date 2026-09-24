use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, State};
use bf_api_provider::types::ScriptsJsonResponse;

pub async fn route(
    State(state): State<AppState>,
    Path(script_hash): Path<String>,
) -> ApiResult<ScriptsJsonResponse> {
    let data_node = state.data_node()?;

    data_node.scripts().json(&script_hash).await
}
