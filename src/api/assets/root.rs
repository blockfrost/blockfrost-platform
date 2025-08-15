use crate::BlockfrostError;
use api_provider::types::AssetsResponse;
use axum::{Json, extract::Query};
use common::pagination::{Pagination, PaginationQuery};

pub async fn route(
    Query(pagination_query): Query<PaginationQuery>,
) -> Result<Json<Vec<AssetsResponse>>, BlockfrostError> {
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
