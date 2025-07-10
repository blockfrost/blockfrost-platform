use crate::api::ApiResult;
use axum::{Extension, extract::Path};
use blockfrost_openapi::models::tx_content::TxContent;
use common::txs::TxsPath;
use dolos::client::Dolos;

pub async fn route(
    Extension(dolos): Extension<Dolos>,
    Path(path): Path<TxsPath>,
) -> ApiResult<TxContent> {
    dolos.txs_hash(&path.hash).await
}
