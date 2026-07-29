use crate::addresses::{AddressInfo, AddressesPath};
use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, State};
use bf_api_provider::types::AddressesContentTotalResponse;

pub async fn route(
    Path(address_path): Path<AddressesPath>,
    State(state): State<AppState>,
) -> ApiResult<AddressesContentTotalResponse> {
    let AddressesPath { address, asset: _ } = address_path;
    let address_info = AddressInfo::from_address(&address, state.config.network.clone())?;
    let data_node = state.data_node()?;

    data_node.addresses().total(&address_info.address).await
}
