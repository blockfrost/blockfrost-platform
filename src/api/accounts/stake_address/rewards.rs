use crate::{BlockfrostError, api::ApiResult, server::state::AppState};
use api_provider::types::AccountsRewardsResponse;
use axum::extract::{Path, Query, State};
use common::{
    accounts::{AccountData, AccountsPath},
    pagination::{Pagination, PaginationQuery},
};

pub async fn route(
    Path(path): Path<AccountsPath>,
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<AccountsRewardsResponse> {
    let account = AccountData::from_account_path(path.stake_address, &state.config.network)?;
    let pagination = Pagination::from_query(pagination_query)?;
    let dolos = state.get_dolos()?;

    dolos
        .accounts()
        .rewards(&account.stake_address, &pagination)
        .await
}
