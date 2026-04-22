use crate::dreps::DrepsPath;
use crate::server::state::AppState;
use axum::extract::{Path, State};
use bf_api_provider::types::DrepsSingleResponse;
use bf_common::types::ApiResult;

pub async fn route(
    Path(drep_path): Path<DrepsPath>,
    State(state): State<AppState>,
) -> ApiResult<DrepsSingleResponse> {
    let data_node = state.data_node()?;

    data_node.governance().drep(&drep_path.drep_id).await
}
