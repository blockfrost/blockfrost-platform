use crate::BlockfrostError;
use axum::extract::Query;
use bf_api_provider::types::AssetsResponse;
use bf_common::{
    pagination::{Pagination, PaginationQuery},
    types::ApiResult,
};

pub async fn route(Query(pagination_query): Query<PaginationQuery>) -> ApiResult<AssetsResponse> {
    let _ = Pagination::from_query(pagination_query)?;

    Err(BlockfrostError::not_found())
}
