use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, State};
use bf_api_provider::types::BlocksSingleResponse;
use bf_common::blocks::BlocksSlotPath;

pub async fn route(
    State(state): State<AppState>,
    Path(blocks_slot_path): Path<BlocksSlotPath>,
) -> ApiResult<BlocksSingleResponse> {
    let dolos = state.get_dolos()?;

    let response = dolos.blocks().by_slot(&blocks_slot_path.slot).await?;

    Ok(response)
}
