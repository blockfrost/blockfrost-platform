use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::BlocksSingleResponse;
use axum::extract::{Path, State};
use common::blocks::BlocksPath;

pub async fn route(
    State(state): State<AppState>,
    Path(blocks_path): Path<BlocksPath>,
) -> ApiResult<BlocksSingleResponse> {
    let dolos = state.get_dolos()?;

    dolos.blocks().by(&blocks_path.hash_or_number).await
}
