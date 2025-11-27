use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, State};
use bf_api_provider::types::TxsCborResponse;
use bf_common::txs::TxsPath;

pub async fn route(
    State(state): State<AppState>,
    Path(path): Path<TxsPath>,
) -> ApiResult<TxsCborResponse> {
    let dolos = state.get_dolos()?;

    dolos.txs().cbor(&path.hash).await
}
