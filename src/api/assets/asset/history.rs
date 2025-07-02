use crate::{
    BlockfrostError,
    api::ApiResult,
    assets::{AssetData, AssetsPath},
};
use axum::extract::{Path, Query};
use blockfrost_openapi::models::asset_history_inner::AssetHistoryInner;
use common::pagination::{Pagination, PaginationQuery};

pub async fn route(
    Path(path): Path<AssetsPath>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<Vec<AssetHistoryInner>> {
    let _ = AssetData::from_query(path.asset)?;
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
