use crate::{
    BlockfrostError,
    accounts::{AccountData, AccountsPath},
    api::ApiResult,
    cli::Config,
};
use axum::{Extension, extract::Path};
use blockfrost_openapi::models::account_addresses_total::AccountAddressesTotal;

pub async fn route(
    Path(path): Path<AccountsPath>,
    Extension(config): Extension<Config>,
) -> ApiResult<AccountAddressesTotal> {
    let _ = AccountData::from_account_path(path.stake_address, config.network)?;

    Err(BlockfrostError::not_found())
}
