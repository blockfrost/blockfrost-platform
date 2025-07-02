use crate::{
    BlockfrostError,
    api::ApiResult,
    assets::{AssetData, AssetsPath},
};
use axum::extract::{Path, Query};
use blockfrost_openapi::models::assets_inner::AssetsInner;
use common::pagination::{Pagination, PaginationQuery};

pub async fn route(
    Path(path): Path<AssetsPath>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<Vec<AssetsInner>> {
    let _ = AssetData::from_query(path.asset)?;
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
