use crate::server::state::AppState;
use axum::extract::{Path, State};
use bf_api_provider::types::DrepsSingleResponse;
use bf_common::{dreps::DrepsPath, types::ApiResult};

pub async fn route(
    Path(drep_path): Path<DrepsPath>,
    State(state): State<AppState>,
) -> ApiResult<DrepsSingleResponse> {
    let dolos = state.get_dolos()?;

    dolos.governance().drep(&drep_path.drep_id).await
}
