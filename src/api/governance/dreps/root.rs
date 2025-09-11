use crate::server::state::AppState;
use api_provider::types::DrepsSingleResponse;
use axum::extract::{Path, State};
use common::{dreps::DrepsPath, errors::BlockfrostError, types::ApiResult};

pub async fn route(
    Path(_drep_path): Path<DrepsPath>,
    State(_state): State<AppState>,
) -> ApiResult<DrepsSingleResponse> {
    Err(BlockfrostError::not_found())
}
