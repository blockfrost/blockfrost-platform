use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::PoolsDelegatorsResponse;
use axum::extract::{Path, Query, State};
use common::{
    pagination::{Pagination, PaginationQuery},
    pools::PoolsPath,
};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(pools_path): Path<PoolsPath>,
) -> ApiResult<PoolsDelegatorsResponse> {
    let pagination = Pagination::from_query(pagination_query).await?;

    state
        .dolos
        .pools()
        .delegators(&pools_path.pool_id, &pagination)
        .await
}
