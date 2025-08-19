use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::BlocksSingleResponse;
use axum::extract::{Path, State};
use common::blocks::BlocksSlotPath;

pub async fn route(
    State(state): State<AppState>,
    Path(blocks_slot_path): Path<BlocksSlotPath>,
) -> ApiResult<BlocksSingleResponse> {
    let response = state.dolos.blocks().by_slot(&blocks_slot_path.slot).await?;

    Ok(response)
}
