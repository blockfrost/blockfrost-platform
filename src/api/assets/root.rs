use crate::BlockfrostError;
use api_provider::types::AssetsResponse;
use axum::extract::Query;
use common::{
    pagination::{Pagination, PaginationQuery},
    types::ApiResult,
};

pub async fn route(Query(pagination_query): Query<PaginationQuery>) -> ApiResult<AssetsResponse> {
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
