use crate::{BlockfrostError, api::ApiResult, config::Config};
use axum::{
    Extension,
    extract::{Path, Query},
};
use blockfrost_openapi::models::address_utxo_content_inner::AddressUtxoContentInner;
use common::{
    addresses::{AddressInfo, AddressesPath},
    pagination::{Pagination, PaginationQuery},
};

pub async fn route(
    Path(address_path): Path<AddressesPath>,
    Extension(config): Extension<Config>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<Vec<AddressUtxoContentInner>> {
    let AddressesPath { address, asset: _ } = address_path;
    let _ = Pagination::from_query(pagination_query).await?;
    let _ = AddressInfo::from_address(&address, config.network)?;

    Err(BlockfrostError::not_found())
}
