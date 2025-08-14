use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::AccountResponse;
use axum::extract::{Path, State};
use common::accounts::{AccountData, AccountsPath};

pub async fn route(
    State(state): State<AppState>,
    Path(path): Path<AccountsPath>,
) -> ApiResult<AccountResponse> {
    let account = AccountData::from_account_path(path.stake_address, &state.config.network)?;

    state
        .api
        .dolos
        .accounts_stake_address(&account.stake_address)
        .await
}
