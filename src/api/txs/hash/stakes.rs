use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, Query, State};
use bf_api_provider::types::TxsStakeAddrResponse;
use bf_common::{
    pagination::{Pagination, PaginationQuery},
    txs::TxsPath,
};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(path): Path<TxsPath>,
) -> ApiResult<TxsStakeAddrResponse> {
    let pagination = Pagination::from_query(pagination_query)?;
    let dolos = state.get_dolos()?;

    dolos.txs().stakes(&path.hash, &pagination).await
}
