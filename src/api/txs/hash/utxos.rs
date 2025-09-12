use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::TxsUtxosResponse;
use axum::extract::{Path, Query, State};
use common::{
    pagination::{Pagination, PaginationQuery},
    txs::TxsPath,
};

pub async fn route(
    State(state): State<AppState>,
    Path(path): Path<TxsPath>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<TxsUtxosResponse> {
    let pagination = Pagination::from_query(pagination_query)?;
    let dolos = state.get_dolos()?;

    dolos.txs().utxos(&path.hash, &pagination).await
}
