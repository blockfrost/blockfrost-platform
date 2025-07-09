use crate::api::ApiResult;
use axum::{
    Extension,
    extract::{Path, Query},
};
use blockfrost_openapi::models::pool_delegators_inner::PoolDelegatorsInner;
use common::{
    pagination::{Pagination, PaginationQuery},
    pools::PoolsPath,
};
use dolos::client::Dolos;

pub async fn route(
    Extension(dolos): Extension<Dolos>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(pools_path): Path<PoolsPath>,
) -> ApiResult<Vec<PoolDelegatorsInner>> {
    let pagination = Pagination::from_query(pagination_query).await?;

    dolos
        .pools_pool_id_delegators(&pools_path.pool_id, &pagination)
        .await
}
