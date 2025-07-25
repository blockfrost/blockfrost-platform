use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, Query, State};
use blockfrost_openapi::models::pool_delegators_inner::PoolDelegatorsInner;
use common::{
    pagination::{Pagination, PaginationQuery},
    pools::PoolsPath,
};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(pools_path): Path<PoolsPath>,
) -> ApiResult<Vec<PoolDelegatorsInner>> {
    let pagination = Pagination::from_query(pagination_query).await?;

    state
        .api
        .dolos
        .pools_pool_id_delegators(&pools_path.pool_id, &pagination)
        .await
}
