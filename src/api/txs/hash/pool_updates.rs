use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, Query, State};
use bf_api_provider::types::TxsPoolCertsResponse;
use bf_common::{
    pagination::{Pagination, PaginationQuery},
    txs::TxsPath,
};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(path): Path<TxsPath>,
) -> ApiResult<TxsPoolCertsResponse> {
    let pagination = Pagination::from_query(pagination_query)?;
    let data_node = state.data_node()?;

    data_node.txs().pool_updates(&path.hash, &pagination).await
}
