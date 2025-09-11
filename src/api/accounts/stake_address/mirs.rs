use crate::{BlockfrostError, api::ApiResult, server::state::AppState};
use api_provider::types::AccountsMirResponse;
use axum::extract::{Path, Query, State};
use common::{
    accounts::{AccountData, AccountsPath},
    pagination::{Pagination, PaginationQuery},
};

pub async fn route(
    Path(path): Path<AccountsPath>,
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<AccountsMirResponse> {
    let _ = AccountData::from_account_path(path.stake_address, &state.config.network)?;
    let _ = Pagination::from_query(pagination_query)?;

    Err(BlockfrostError::not_found())
}
