use crate::{
    accounts::{AccountData, AccountsPath},
    server::state::AppState,
};
use axum::extract::{Path, Query, State};
use bf_api_provider::types::AccountsTransactionsResponse;
use bf_common::{
    pagination::{Pagination, PaginationQuery},
    types::ApiResult,
};

pub async fn route(
    Path(path): Path<AccountsPath>,
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<AccountsTransactionsResponse> {
    let account = AccountData::from_account_path(path.stake_address, &state.config.network)?;
    let pagination = Pagination::from_query(pagination_query)?;
    let data_node = state.data_node()?;

    data_node
        .accounts()
        .transactions(&account.stake_address, &pagination)
        .await
}
