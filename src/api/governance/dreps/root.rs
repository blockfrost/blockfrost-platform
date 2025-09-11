use crate::server::state::AppState;
use api_provider::types::DrepsSingleResponse;
use axum::extract::{Path, State};
use common::{dreps::DrepsPath, errors::BlockfrostError, types::ApiResult};

pub async fn route(
    Path(drep_path): Path<DrepsPath>,
    State(state): State<AppState>,
) -> ApiResult<DrepsSingleResponse> {
    Err(BlockfrostError::not_found())
}
