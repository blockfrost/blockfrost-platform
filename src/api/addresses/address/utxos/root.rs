use crate::server::state::AppState;
use api_provider::types::AddressesUtxosResponse;
use axum::extract::{Path, Query, State};
use common::{
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
    let pagination = Pagination::from_query(pagination_query).await?;
    let address_info = AddressInfo::from_address(&address, state.config.network.clone())?;
    let dolos = state.get_dolos()?;

    dolos
        .addresses()
        .utxos(&address_info.address, &pagination)
        .await
}
