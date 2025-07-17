use crate::api::ApiResult;
use axum::Extension;
use dolos::client::Dolos;

pub async fn route(Extension(dolos): Extension<Dolos>) -> ApiResult<Vec<String>> {
    dolos.blocks_latest_txs().await
}
