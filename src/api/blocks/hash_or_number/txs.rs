use crate::api::ApiResult;
use axum::{
    Extension,
    extract::{Path, Query},
};
use common::{
    blocks::BlocksPath,
    pagination::{Pagination, PaginationQuery},
};
use dolos::client::Dolos;

pub async fn route(
    Extension(dolos): Extension<Dolos>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(blocks_path): Path<BlocksPath>,
) -> ApiResult<Vec<String>> {
    let pagination = Pagination::from_query(pagination_query).await?;
    let response = dolos
        .blocks_hash_or_number_txs(&blocks_path.hash_or_number, pagination)
        .await?;

    Ok(response)
}
