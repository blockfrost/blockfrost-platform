use crate::{
    BlockfrostError,
    accounts::{AccountData, AccountsPath},
    api::ApiResult,
    pagination::{Pagination, PaginationQuery},
    server::state::AppState,
};
use axum::extract::{Path, Query, State};
use blockfrost_openapi::models::account_history_content_inner::AccountHistoryContentInner;

pub async fn route(
    State(state): State<AppState>,
    Path(path): Path<AccountsPath>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<Vec<AccountHistoryContentInner>> {
    let _ = AccountData::from_account_path(path.stake_address, &state.config.network)?;
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
