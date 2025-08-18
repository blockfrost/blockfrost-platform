use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::TxsMirsResponse;
use axum::extract::{Path, Query, State};
use common::{
    pagination::{Pagination, PaginationQuery},
    txs::TxsPath,
};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(path): Path<TxsPath>,
) -> ApiResult<TxsMirsResponse> {
    let pagination = Pagination::from_query(pagination_query).await?;

    state.api.dolos.txs_hash_mirs(&path.hash, &pagination).await
}
