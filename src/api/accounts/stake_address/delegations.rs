use crate::{api::ApiResult, server::state::AppState};
use axum::{
    Extension,
    extract::{Path, Query, State},
};
use blockfrost_openapi::models::account_delegation_content_inner::AccountDelegationContentInner;
use common::{
    accounts::{AccountData, AccountsPath},
    pagination::{Pagination, PaginationQuery},
};
use dolos::client::Dolos;

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Extension(dolos): Extension<Dolos>,
    Path(path): Path<AccountsPath>,
) -> ApiResult<Vec<AccountDelegationContentInner>> {
    let account = AccountData::from_account_path(path.stake_address, &state.config.network)?;
    let pagination = Pagination::from_query(pagination_query).await?;

    dolos
        .accounts_stake_address_delegations(&account.stake_address, &pagination)
        .await
}
