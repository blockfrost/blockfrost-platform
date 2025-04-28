use crate::{
    BlockfrostError,
    api::ApiResult,
    pagination::{Pagination, PaginationQuery},
};
use axum::extract::Query;
use blockfrost_openapi::models::assets_inner::AssetsInner;

pub async fn route(Query(pagination_query): Query<PaginationQuery>) -> ApiResult<Vec<AssetsInner>> {
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
