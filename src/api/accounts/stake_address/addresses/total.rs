use crate::{
    BlockfrostError,
    accounts::{AccountData, AccountsPath},
    api::ApiResult,
    server::state::AppState,
};
use axum::extract::{Path, State};
use blockfrost_openapi::models::account_addresses_total::AccountAddressesTotal;

pub async fn route(
    Path(path): Path<AccountsPath>,
    State(state): State<AppState>,
) -> ApiResult<AccountAddressesTotal> {
    let _ = AccountData::from_account_path(path.stake_address, &state.config.network)?;

    Err(BlockfrostError::not_found())
}
