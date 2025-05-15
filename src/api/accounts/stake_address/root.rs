use crate::{
    BlockfrostError,
    accounts::{AccountData, AccountsPath},
    api::ApiResult,
    config::Config,
};

use axum::{Extension, extract::Path};
use blockfrost_openapi::models::account_content::AccountContent;

pub async fn route(
    Path(path): Path<AccountsPath>,
    Extension(config): Extension<Config>,
) -> ApiResult<AccountContent> {
    let _ = AccountData::from_account_path(path.stake_address, config.network)?;

    Err(BlockfrostError::not_found())
}
