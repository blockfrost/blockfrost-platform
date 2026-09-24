use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, State};
use bf_api_provider::types::ScriptsDatumResponse;

pub async fn route(
    State(state): State<AppState>,
    Path(datum_hash): Path<String>,
) -> ApiResult<ScriptsDatumResponse> {
    let data_node = state.data_node()?;

    data_node.scripts().datum(&datum_hash).await
}
