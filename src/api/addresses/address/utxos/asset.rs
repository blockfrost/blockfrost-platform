use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::AddressesUtxosAssetResponse;
use axum::extract::{Path, Query, State};
use common::{
    addresses::{AddressInfo, AddressesPath},
    errors::BlockfrostError,
    pagination::{Pagination, PaginationQuery},
};

pub async fn route(
    Path(address_path): Path<AddressesPath>,
    State(app_state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    State(state): State<AppState>,
) -> ApiResult<AddressesUtxosAssetResponse> {
    let AddressesPath { address, asset } = address_path;
    let pagination = Pagination::from_query(pagination_query)?;
    let address_info = AddressInfo::from_address(&address, app_state.config.network.clone())?;
    let dolos = state.get_dolos()?;
    let asset = asset.ok_or(BlockfrostError::invalid_asset_name())?;

    dolos
        .addresses()
        .utxos_asset(&address_info.address, &asset, &pagination)
        .await
}
