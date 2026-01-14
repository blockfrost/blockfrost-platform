use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, Query, State};
use bf_api_provider::types::BlocksResponse;
use bf_common::{
    blocks::{BlockData, BlocksPath},
    pagination::{Pagination, PaginationQuery},
};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(blocks_path): Path<BlocksPath>,
) -> ApiResult<BlocksResponse> {
    let block_data = BlockData::from_string(blocks_path.hash_or_number)?;
    let pagination = Pagination::from_query(pagination_query)?;
    let data_node = state.get_data_node()?;

    data_node
        .blocks()
        .previous(&block_data.hash_or_number, &pagination)
        .await
}
