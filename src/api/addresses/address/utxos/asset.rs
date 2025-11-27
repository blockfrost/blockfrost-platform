use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, Query, State};
use bf_api_provider::types::AddressesUtxosAssetResponse;
use bf_common::{
    addresses::{AddressInfo, AddressPathWithAsset},
    assets::AssetData,
    pagination::{Pagination, PaginationQuery},
};

pub async fn route(
    Path(address_path_with_asset): Path<AddressPathWithAsset>,
    State(app_state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    State(state): State<AppState>,
) -> ApiResult<AddressesUtxosAssetResponse> {
    let AddressPathWithAsset { address, asset } = address_path_with_asset;
    let pagination = Pagination::from_query(pagination_query)?;
    let address_info = AddressInfo::from_address(&address, app_state.config.network.clone())?;
    let dolos = state.get_dolos()?;
    let asset_data = AssetData::from_query(asset)?;

    dolos
        .addresses()
        .utxos_asset(&address_info.address, &asset_data.asset, &pagination)
        .await
}
