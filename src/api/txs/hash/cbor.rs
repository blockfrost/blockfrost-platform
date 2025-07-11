use crate::api::ApiResult;
use axum::{Extension, extract::Path};
use blockfrost_openapi::models::tx_content_cbor::TxContentCbor;
use common::txs::TxsPath;
use dolos::client::Dolos;

pub async fn route(
    Extension(dolos): Extension<Dolos>,
    Path(path): Path<TxsPath>,
) -> ApiResult<TxContentCbor> {
    dolos.txs_hash_cbor(&path.hash).await
}
