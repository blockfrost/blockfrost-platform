use crate::api::ApiResult;
use axum::{
    Extension,
    extract::{Path, Query},
};
use blockfrost_openapi::models::tx_content_pool_certs_inner::TxContentPoolCertsInner;
use common::{
    pagination::{Pagination, PaginationQuery},
    txs::TxsPath,
};
use dolos::client::Dolos;

pub async fn route(
    Extension(dolos): Extension<Dolos>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(path): Path<TxsPath>,
) -> ApiResult<Vec<TxContentPoolCertsInner>> {
    let pagination = Pagination::from_query(pagination_query).await?;

    dolos.txs_hash_pool_updates(&path.hash, &pagination).await
}
