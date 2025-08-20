use crate::{BlockfrostError, api::ApiResult, server::state::AppState};
use api_provider::types::AccountsAddressesTotalResponse;
use axum::extract::{Path, State};
use common::accounts::{AccountData, AccountsPath};

pub async fn route(
    Path(path): Path<AccountsPath>,
    State(state): State<AppState>,
) -> ApiResult<AccountsAddressesTotalResponse> {
    let _ = AccountData::from_account_path(path.stake_address, &state.config.network)?;

    Err(BlockfrostError::not_found())
}
