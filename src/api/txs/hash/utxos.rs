use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, State};
use blockfrost_openapi::models::tx_content_utxo::TxContentUtxo;
use common::txs::TxsPath;

pub async fn route(
    State(state): State<AppState>,
    Path(path): Path<TxsPath>,
) -> ApiResult<TxContentUtxo> {
    state.api.dolos.txs_hash_utxos(&path.hash).await
}
