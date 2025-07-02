use crate::BlockfrostError;
use axum::{Json, extract::Query};
use blockfrost_openapi::models::assets_inner::AssetsInner;
use common::pagination::{Pagination, PaginationQuery};

pub async fn route(
    Query(pagination_query): Query<PaginationQuery>,
) -> Result<Json<Vec<AssetsInner>>, BlockfrostError> {
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
