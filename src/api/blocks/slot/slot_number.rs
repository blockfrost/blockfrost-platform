use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, State};
use blockfrost_openapi::models::block_content::BlockContent;
use common::blocks::BlocksSlotPath;

pub async fn route(
    State(state): State<AppState>,
    Path(blocks_slot_path): Path<BlocksSlotPath>,
) -> ApiResult<BlockContent> {
    let response = state
        .api
        .dolos
        .blocks_slot_slot(&blocks_slot_path.slot)
        .await?;

    Ok(response)
}
