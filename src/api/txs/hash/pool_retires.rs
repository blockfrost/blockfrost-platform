use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::TxsPoolRetiresResponse;
use axum::extract::{Path, Query, State};
use common::{
    pagination::{Pagination, PaginationQuery},
    txs::TxsPath,
};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(path): Path<TxsPath>,
) -> ApiResult<TxsPoolRetiresResponse> {
    let pagination = Pagination::from_query(pagination_query).await?;

    state
        .dolos
        .txs()
        .pool_retires(&path.hash, &pagination)
        .await
}
