use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::AccountsAddressesResponse;
use axum::extract::{Path, Query, State};
use common::{
    accounts::{AccountData, AccountsPath},
    pagination::{Pagination, PaginationQuery},
};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(path): Path<AccountsPath>,
) -> ApiResult<AccountsAddressesResponse> {
    let account = AccountData::from_account_path(path.stake_address, &state.config.network)?;
    let pagination = Pagination::from_query(pagination_query).await?;

    state
        .api
        .dolos
        .accounts_stake_address_addresses(&account.stake_address, &pagination)
        .await
}
