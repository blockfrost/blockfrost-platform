use crate::{BlockfrostError, api::ApiResult};
use api_provider::types::AddressesUtxosResponse;
use axum::{
    Extension,
    extract::{Path, Query},
};
use common::{
    addresses::{AddressInfo, AddressesPath},
    config::Config,
    pagination::{Pagination, PaginationQuery},
};

pub async fn route(
    Path(address_path): Path<AddressesPath>,
    Extension(config): Extension<Config>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<AddressesUtxosResponse> {
    let AddressesPath { address, asset: _ } = address_path;
    let _ = Pagination::from_query(pagination_query).await?;
    let _ = AddressInfo::from_address(&address, config.network)?;

    Err(BlockfrostError::not_found())
}
