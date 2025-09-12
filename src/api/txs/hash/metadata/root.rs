use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::TxsMetadataResponse;
use axum::extract::{Path, Query, State};
use common::{
    pagination::{Pagination, PaginationQuery},
    txs::TxsPath,
};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(path): Path<TxsPath>,
) -> ApiResult<TxsMetadataResponse> {
    let pagination = Pagination::from_query(pagination_query)?;
    let dolos = state.get_dolos()?;

    dolos.txs().metadata(&path.hash, &pagination).await
}
