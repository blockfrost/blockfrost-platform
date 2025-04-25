use crate::{
    BlockfrostError,
    accounts::{AccountData, AccountsPath},
    api::ApiResult,
    cli::Config,
    pagination::{Pagination, PaginationQuery},
};

use axum::{
    Extension,
    extract::{Path, Query},
};
use blockfrost_openapi::models::account_delegation_content_inner::AccountDelegationContentInner;

pub async fn route(
    Extension(config): Extension<Config>,
    Query(pagination_query): Query<PaginationQuery>,
    Path(path): Path<AccountsPath>,
) -> ApiResult<Vec<AccountDelegationContentInner>> {
    let _ = AccountData::from_account_path(path.stake_address, config.network)?;
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
