use crate::{
    BlockfrostError,
    accounts::{AccountData, AccountsPath},
    api::ApiResult,
    pagination::{Pagination, PaginationQuery},
    server::AppState,
};
use axum::extract::{Path, Query, State};
use blockfrost_openapi::models::account_delegation_content_inner::AccountDelegationContentInner;

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(path): Path<AccountsPath>,
) -> ApiResult<Vec<AccountDelegationContentInner>> {
    let _ = AccountData::from_account_path(path.stake_address, &state.config.network)?;
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
