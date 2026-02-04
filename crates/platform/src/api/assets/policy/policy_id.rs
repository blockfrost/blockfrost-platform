use crate::{BlockfrostError, api::ApiResult};
use axum::extract::Query;
use bf_api_provider::types::AssetsPolicyResponse;
use bf_common::pagination::{Pagination, PaginationQuery};

pub async fn route(
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<AssetsPolicyResponse> {
    let _ = Pagination::from_query(pagination_query)?;

    Err(BlockfrostError::not_found())
}
