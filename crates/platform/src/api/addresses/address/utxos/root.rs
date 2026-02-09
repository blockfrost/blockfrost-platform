use crate::server::state::AppState;
use axum::extract::{Path, Query, State};
use bf_api_provider::types::AddressesUtxosResponse;
use bf_common::{
    addresses::{AddressInfo, AddressesPath},
    pagination::{Pagination, PaginationQuery},
    types::ApiResult,
};

pub async fn route(
    State(state): State<AppState>,
    Path(address_path): Path<AddressesPath>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<AddressesUtxosResponse> {
    let AddressesPath { address, asset: _ } = address_path;
    let pagination = Pagination::from_query(pagination_query)?;
    let address_info = AddressInfo::from_address(&address, state.config.network.clone())?;
    let data_node = state.data_node()?;

    data_node
        .addresses()
        .utxos(&address_info.address, &pagination)
        .await
}
