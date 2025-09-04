use crate::{BlockfrostError, api::ApiResult, server::state::AppState};
use axum::extract::{Path, State};
use common::addresses::{AddressInfo, AddressesPath};

pub async fn route(
    Path(address_path): Path<AddressesPath>,
    State(state): State<AppState>,
) -> ApiResult<()> {
    let AddressesPath { address, asset: _ } = address_path;
    let _ = AddressInfo::from_address(&address, state.config.network.clone())?;

    Err(BlockfrostError::not_found())
}
