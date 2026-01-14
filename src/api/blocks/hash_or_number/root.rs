use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, State};
use bf_api_provider::types::BlocksSingleResponse;
use bf_common::blocks::{BlockData, BlocksPath};

pub async fn route(
    State(state): State<AppState>,
    Path(blocks_path): Path<BlocksPath>,
) -> ApiResult<BlocksSingleResponse> {
    let block_data = BlockData::from_string(blocks_path.hash_or_number)?;
    let data_node = state.get_data_node()?;

    data_node.blocks().by(&block_data.hash_or_number).await
}
