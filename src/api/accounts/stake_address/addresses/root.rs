use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, Query, State};
use bf_api_provider::types::AccountsAddressesResponse;
use bf_common::{
    accounts::{AccountData, AccountsPath},
    pagination::{Pagination, PaginationQuery},
};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(path): Path<AccountsPath>,
) -> ApiResult<AccountsAddressesResponse> {
    let account = AccountData::from_account_path(path.stake_address, &state.config.network)?;
    let pagination = Pagination::from_query(pagination_query)?;
    let data_node = state.get_data_node()?;

    data_node
        .accounts()
        .addresses(&account.stake_address, &pagination)
        .await
}
