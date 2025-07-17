use crate::{api::ApiResult, server::state::AppState};
use axum::{
    Extension,
    extract::{Path, State},
};
use blockfrost_openapi::models::account_content::AccountContent;
use common::accounts::{AccountData, AccountsPath};
use dolos::client::Dolos;

pub async fn route(
    State(state): State<AppState>,
    Extension(dolos): Extension<Dolos>,
    Path(path): Path<AccountsPath>,
) -> ApiResult<AccountContent> {
    let account = AccountData::from_account_path(path.stake_address, &state.config.network)?;

    dolos.accounts_stake_address(&account.stake_address).await
}
