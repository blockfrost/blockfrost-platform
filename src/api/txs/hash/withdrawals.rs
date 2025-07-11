use crate::api::ApiResult;
use axum::{
    Extension,
    extract::{Path, Query},
};
use blockfrost_openapi::models::tx_content_withdrawals_inner::TxContentWithdrawalsInner;
use common::{
    pagination::{Pagination, PaginationQuery},
    txs::TxsPath,
};
use dolos::client::Dolos;

pub async fn route(
    Extension(dolos): Extension<Dolos>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(path): Path<TxsPath>,
) -> ApiResult<Vec<TxContentWithdrawalsInner>> {
    let pagination = Pagination::from_query(pagination_query).await?;

    dolos.txs_hash_withdrawals(&path.hash, &pagination).await
}
