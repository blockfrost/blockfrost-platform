use crate::server::state::AppState;
use api_provider::types::AddressesUtxosResponse;
use axum::{
    Extension,
    extract::{Path, Query, State},
};
use common::{
    addresses::{AddressInfo, AddressesPath},
    config::Config,
    pagination::{Pagination, PaginationQuery},
    types::ApiResult,
};

pub async fn route(
    State(state): State<AppState>,
    Path(address_path): Path<AddressesPath>,
    Extension(config): Extension<Config>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<AddressesUtxosResponse> {
    let AddressesPath { address, asset: _ } = address_path;
    let pagination = Pagination::from_query(pagination_query).await?;
    let address_info = AddressInfo::from_address(&address, config.network)?;

    state
        .api
        .dolos
        .addresses_address_utxos(&address_info.address, &pagination)
        .await
}
