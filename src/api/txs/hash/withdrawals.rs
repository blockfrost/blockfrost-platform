use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::TxsWithdrawalsResponse;
use axum::extract::{Path, Query, State};
use common::{
    pagination::{Pagination, PaginationQuery},
    txs::TxsPath,
};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(path): Path<TxsPath>,
) -> ApiResult<TxsWithdrawalsResponse> {
    let pagination = Pagination::from_query(pagination_query).await?;
    let dolos = state.get_dolos()?;

    dolos.txs().withdrawals(&path.hash, &pagination).await
}
