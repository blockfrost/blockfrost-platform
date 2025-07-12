use crate::api::ApiResult;
use axum::{
    Extension,
    extract::{Path, Query},
};
use blockfrost_openapi::models::block_content::BlockContent;
use common::{
    blocks::BlocksPath,
    pagination::{Pagination, PaginationQuery},
};
use dolos::client::Dolos;

pub async fn route(
    Extension(dolos): Extension<Dolos>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(blocks_path): Path<BlocksPath>,
) -> ApiResult<Vec<BlockContent>> {
    let pagination = Pagination::from_query(pagination_query).await?;

    dolos
        .blocks_next(&blocks_path.hash_or_number, &pagination)
        .await
}
