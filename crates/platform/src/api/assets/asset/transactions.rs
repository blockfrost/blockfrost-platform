use crate::{BlockfrostError, api::ApiResult};
use axum::extract::{Path, Query};
use bf_api_provider::types::AssetsTransactionsResponse;
use bf_common::{
    assets::{AssetData, AssetsPath},
    pagination::{Pagination, PaginationQuery},
};

pub async fn route(
    Path(path): Path<AssetsPath>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<AssetsTransactionsResponse> {
    let _ = AssetData::from_query(path.asset)?;
    let _ = Pagination::from_query(pagination_query)?;

    Err(BlockfrostError::not_found())
}
