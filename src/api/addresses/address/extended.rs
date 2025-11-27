use crate::{BlockfrostError, api::ApiResult, server::state::AppState};
use axum::extract::{Path, State};
use bf_api_provider::types::AddressesContentExtendedResponse;
use bf_common::addresses::{AddressInfo, AddressesPath};

pub async fn route(
    Path(address_path): Path<AddressesPath>,
    State(app_state): State<AppState>,
) -> ApiResult<AddressesContentExtendedResponse> {
    let AddressesPath { address, asset: _ } = address_path;
    let _ = AddressInfo::from_address(&address, app_state.config.network.clone())?;

    Err(BlockfrostError::not_found())
}
