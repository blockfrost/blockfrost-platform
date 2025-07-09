use axum::{
    Extension,
    extract::{Path, Query},
};
use blockfrost_openapi::models::address_utxo_content_inner::AddressUtxoContentInner;
use common::{
    addresses::{AddressInfo, AddressesPath},
    config::Config,
    pagination::{Pagination, PaginationQuery},
    types::ApiResult,
};
use dolos::client::Dolos;

pub async fn route(
    Path(address_path): Path<AddressesPath>,
    Extension(config): Extension<Config>,
    Query(pagination_query): Query<PaginationQuery>,
    Extension(dolos): Extension<Dolos>,
) -> ApiResult<Vec<AddressUtxoContentInner>> {
    let AddressesPath { address, asset: _ } = address_path;
    let pagination = Pagination::from_query(pagination_query).await?;
    let address_info = AddressInfo::from_address(&address, config.network)?;

    let utxos = dolos
        .addresses_address_utxos(&address_info.address, &pagination)
        .await?;

    Ok(utxos)
}
