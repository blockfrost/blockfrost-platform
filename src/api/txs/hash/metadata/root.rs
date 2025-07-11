use crate::api::ApiResult;
use axum::{
    Extension,
    extract::{Path, Query},
};
use blockfrost_openapi::models::tx_content_metadata_inner::TxContentMetadataInner;
use common::{
    pagination::{Pagination, PaginationQuery},
    txs::TxsPath,
};
use dolos::client::Dolos;

pub async fn route(
    Extension(dolos): Extension<Dolos>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(path): Path<TxsPath>,
) -> ApiResult<Vec<TxContentMetadataInner>> {
    let pagination = Pagination::from_query(pagination_query).await?;

    dolos.txs_hash_metadata(&path.hash, &pagination).await
}
