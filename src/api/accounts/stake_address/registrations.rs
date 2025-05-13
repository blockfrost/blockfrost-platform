use crate::{
    BlockfrostError,
    accounts::{AccountData, AccountsPath},
    api::ApiResult,
    pagination::{Pagination, PaginationQuery},
    server::AppState,
};
use axum::extract::{Path, Query, State};
use blockfrost_openapi::models::account_registration_content_inner::AccountRegistrationContentInner;

pub async fn route(
    Path(path): Path<AccountsPath>,
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<Vec<AccountRegistrationContentInner>> {
    let _ = AccountData::from_account_path(path.stake_address, &state.config.network)?;
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
