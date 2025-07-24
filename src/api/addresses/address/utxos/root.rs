use axum::{
    Extension,
    extract::{Path, Query, State},
};
use blockfrost_openapi::models::address_utxo_content_inner::AddressUtxoContentInner;
use common::{
    addresses::{AddressInfo, AddressesPath},
    config::Config,
    pagination::{Pagination, PaginationQuery},
    types::ApiResult,
};

use crate::server::state::AppState;

pub async fn route(
    State(state): State<AppState>,
    Path(address_path): Path<AddressesPath>,
    Extension(config): Extension<Config>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<Vec<AddressUtxoContentInner>> {
    let AddressesPath { address, asset: _ } = address_path;
    let pagination = Pagination::from_query(pagination_query).await?;
    let address_info = AddressInfo::from_address(&address, config.network)?;

    state
        .api
        .dolos
        .addresses_address_utxos(&address_info.address, &pagination)
        .await
}
