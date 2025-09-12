use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::AssetsPolicyResponse;
use axum::extract::Query;
use common::pagination::{Pagination, PaginationQuery};

pub async fn route(
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<AssetsPolicyResponse> {
    let _ = Pagination::from_query(pagination_query)?;

    Err(BlockfrostError::not_found())
}
