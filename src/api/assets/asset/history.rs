use crate::{
    BlockfrostError,
    api::ApiResult,
    pagination::{Pagination, PaginationQuery},
};
use axum::extract::Query;
use blockfrost_openapi::models::asset_history_inner::AssetHistoryInner;

pub async fn route(
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<Vec<AssetHistoryInner>> {
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
