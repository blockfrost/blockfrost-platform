use crate::{BlockfrostError, api::ApiResult};

use blockfrost_openapi::models::account_history_content_inner::AccountHistoryContentInner;

pub async fn route() -> ApiResult<Vec<AccountHistoryContentInner>> {
    Err(BlockfrostError::not_found())
}
