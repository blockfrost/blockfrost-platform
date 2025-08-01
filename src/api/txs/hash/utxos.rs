use crate::api::ApiResult;
use axum::{Extension, extract::Path};
use blockfrost_openapi::models::tx_content_utxo::TxContentUtxo;
use common::txs::TxsPath;
use dolos::client::Dolos;

pub async fn route(
    Extension(dolos): Extension<Dolos>,
    Path(path): Path<TxsPath>,
) -> ApiResult<TxContentUtxo> {
    dolos.txs_hash_utxos(&path.hash).await
}
