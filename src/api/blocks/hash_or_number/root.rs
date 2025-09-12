use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::BlocksSingleResponse;
use axum::extract::{Path, State};
use common::blocks::{BlockData, BlocksPath};

pub async fn route(
    State(state): State<AppState>,
    Path(blocks_path): Path<BlocksPath>,
) -> ApiResult<BlocksSingleResponse> {
    let block_data = BlockData::from_string(blocks_path.hash_or_number)?;
    let dolos = state.get_dolos()?;

    dolos.blocks().by(&block_data.hash_or_number).await
}
