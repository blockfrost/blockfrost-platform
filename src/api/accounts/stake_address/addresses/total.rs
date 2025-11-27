use crate::{BlockfrostError, api::ApiResult, server::state::AppState};
use axum::extract::{Path, State};
use bf_api_provider::types::AccountsAddressesTotalResponse;
use bf_common::accounts::{AccountData, AccountsPath};

pub async fn route(
    Path(path): Path<AccountsPath>,
    State(state): State<AppState>,
) -> ApiResult<AccountsAddressesTotalResponse> {
    let _ = AccountData::from_account_path(path.stake_address, &state.config.network)?;

    Err(BlockfrostError::not_found())
}
