use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, State};
use bf_api_provider::types::AccountsResponse;
use bf_common::accounts::{AccountData, AccountsPath};

pub async fn route(
    State(state): State<AppState>,
    Path(path): Path<AccountsPath>,
) -> ApiResult<AccountsResponse> {
    let account = AccountData::from_account_path(path.stake_address, &state.config.network)?;
    let data_node = state.get_data_node()?;

    data_node
        .accounts()
        .stake_address(&account.stake_address)
        .await
}
