use crate::{
    BlockfrostError,
    api::ApiResult,
    pagination::{Pagination, PaginationQuery},
};
use axum::extract::Query;
use blockfrost_openapi::models::asset_policy_inner::AssetPolicyInner;

pub async fn route(
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<Vec<AssetPolicyInner>> {
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
