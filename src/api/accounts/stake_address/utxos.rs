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
use blockfrost_openapi::models::account_utxo_content_inner::AccountUtxoContentInner;

pub async fn route(
    Path(path): Path<AccountsPath>,
    Extension(config): Extension<Config>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<Vec<AccountUtxoContentInner>> {
    let _ = AccountData::from_account_path(path.stake_address, config.network)?;
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
