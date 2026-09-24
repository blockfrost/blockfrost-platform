use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, State};
use bf_api_provider::types::ScriptsDatumCborResponse;

pub async fn route(
    State(state): State<AppState>,
    Path(datum_hash): Path<String>,
) -> ApiResult<ScriptsDatumCborResponse> {
    let data_node = state.data_node()?;

    data_node.scripts().datum_cbor(&datum_hash).await
}
