use crate::pools::{PoolData, PoolsPath};
use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, Query, State};
use bf_api_provider::types::PoolsHistoryResponse;
use bf_common::pagination::{Pagination, PaginationQuery};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(pools_path): Path<PoolsPath>,
) -> ApiResult<PoolsHistoryResponse> {
    let pool_data = PoolData::from_path(&pools_path.pool_id)?;
    let pagination = Pagination::from_query(pagination_query)?;
    let data_node = state.data_node()?;

    data_node
        .pools()
        .history(&pool_data.pool_id, &pagination)
        .await
}
