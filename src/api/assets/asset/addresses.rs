use crate::{BlockfrostError, api::ApiResult};
use axum::extract::Query;
use blockfrost_openapi::models::asset_addresses_inner::AssetAddressesInner;
use common::pagination::{Pagination, PaginationQuery};

pub async fn route(
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<AssetAddressesInner> {
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
