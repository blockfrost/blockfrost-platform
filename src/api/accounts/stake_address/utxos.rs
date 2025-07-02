use crate::{
    BlockfrostError,
    accounts::{AccountData, AccountsPath},
    api::ApiResult,
    server::state::AppState,
};
use axum::extract::{Path, Query, State};
use blockfrost_openapi::models::account_utxo_content_inner::AccountUtxoContentInner;
use common::pagination::{Pagination, PaginationQuery};

pub async fn route(
    Path(path): Path<AccountsPath>,
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<Vec<AccountUtxoContentInner>> {
    let _ = AccountData::from_account_path(path.stake_address, &state.config.network)?;
    let _ = Pagination::from_query(pagination_query).await?;

    Err(BlockfrostError::not_found())
}
