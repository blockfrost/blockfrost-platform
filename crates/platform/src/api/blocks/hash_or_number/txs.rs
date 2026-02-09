use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, Query, State};
use bf_common::{
    blocks::{BlockData, BlocksPath},
    pagination::{Pagination, PaginationQuery},
};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(blocks_path): Path<BlocksPath>,
) -> ApiResult<Vec<String>> {
    let block_data = BlockData::from_string(blocks_path.hash_or_number)?;
    let pagination = Pagination::from_query(pagination_query)?;
    let dolos = state.get_dolos()?;

    dolos
        .blocks()
        .txs(&block_data.hash_or_number, &pagination)
        .await
}
