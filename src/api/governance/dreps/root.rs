use crate::server::state::AppState;
use api_provider::types::DrepsSingleResponse;
use axum::extract::{Path, State};
use common::{dreps::DrepsPath, types::ApiResult};

pub async fn route(
    Path(drep_path): Path<DrepsPath>,
    State(state): State<AppState>,
) -> ApiResult<DrepsSingleResponse> {
    state
        .api
        .dolos
        .governance_dreps_drep_id(&drep_path.drep_id)
        .await
}
