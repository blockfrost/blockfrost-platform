use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, State};
use blockfrost_openapi::models::tx_content_cbor::TxContentCbor;
use common::txs::TxsPath;

pub async fn route(
    State(state): State<AppState>,
    Path(path): Path<TxsPath>,
) -> ApiResult<TxContentCbor> {
    state.api.dolos.txs_hash_cbor(&path.hash).await
}
