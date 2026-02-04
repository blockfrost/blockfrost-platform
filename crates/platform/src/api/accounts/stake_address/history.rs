use crate::{BlockfrostError, api::ApiResult, server::state::AppState};
use axum::extract::{Path, Query, State};
use bf_api_provider::types::AccountsHistoryResponse;
use bf_common::{
    accounts::{AccountData, AccountsPath},
    pagination::{Pagination, PaginationQuery},
};

pub async fn route(
    State(state): State<AppState>,
    Path(path): Path<AccountsPath>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<AccountsHistoryResponse> {
    let _ = AccountData::from_account_path(path.stake_address, &state.config.network)?;
    let _ = Pagination::from_query(pagination_query)?;

    Err(BlockfrostError::not_found())
}
