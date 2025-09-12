use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::PoolsDelegatorsResponse;
use axum::extract::{Path, Query, State};
use common::{
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
    let dolos = state.get_dolos()?;

    dolos
        .pools()
        .delegators(&pool_data.pool_id, &pagination)
        .await
}
