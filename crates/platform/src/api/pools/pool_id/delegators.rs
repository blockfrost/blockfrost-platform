use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, Query, State};
use bf_api_provider::types::PoolsDelegatorsResponse;
use bf_common::{
    pagination::{Pagination, PaginationQuery},
    pools::{PoolData, PoolsPath},
};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(pools_path): Path<PoolsPath>,
) -> ApiResult<PoolsDelegatorsResponse> {
    let pool_data = PoolData::from_path(&pools_path.pool_id)?;
    let pagination = Pagination::from_query(pagination_query)?;
    let data_node = state.data_node()?;

    data_node
        .pools()
        .delegators(&pool_data.pool_id, &pagination)
        .await
}
