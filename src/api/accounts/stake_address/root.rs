use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, State};
use blockfrost_openapi::models::account_content::AccountContent;
use common::accounts::{AccountData, AccountsPath};

pub async fn route(
    State(state): State<AppState>,
    Path(path): Path<AccountsPath>,
) -> ApiResult<AccountContent> {
    let account = AccountData::from_account_path(path.stake_address, &state.config.network)?;

    state
        .api
        .dolos
        .accounts_stake_address(&account.stake_address)
        .await
}
