use crate::{
    BlockfrostError,
    accounts::{AccountData, AccountsPath},
    api::ApiResult,
    config::Config,
    pagination::{Pagination, PaginationQuery},
};
use axum::{
    Extension,
    extract::{Path, Query},
};
use blockfrost_openapi::models::account_mir_content_inner::AccountMirContentInner;

pub async fn route(
    Path(path): Path<AccountsPath>,
    Extension(config): Extension<Config>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<Vec<AccountMirContentInner>> {
    let _ = AccountData::from_account_path(path.stake_address, config.network)?;
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
