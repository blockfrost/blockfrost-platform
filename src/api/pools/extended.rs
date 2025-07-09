use crate::api::ApiResult;
use axum::Extension;
use blockfrost_openapi::models::pool_list_extended_inner::PoolListExtendedInner;
use dolos::client::Dolos;

pub async fn route(Extension(dolos): Extension<Dolos>) -> ApiResult<Vec<PoolListExtendedInner>> {
    dolos.pools_extended().await
}
