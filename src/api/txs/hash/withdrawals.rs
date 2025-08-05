use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, Query, State};
use blockfrost_openapi::models::tx_content_withdrawals_inner::TxContentWithdrawalsInner;
use common::{
    pagination::{Pagination, PaginationQuery},
    txs::TxsPath,
};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(path): Path<TxsPath>,
) -> ApiResult<Vec<TxContentWithdrawalsInner>> {
    let pagination = Pagination::from_query(pagination_query).await?;

    state
        .api
        .dolos
        .txs_hash_withdrawals(&path.hash, &pagination)
        .await
}
