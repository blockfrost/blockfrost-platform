use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::TxsUtxosResponse;
use axum::extract::{Path, State};
use common::txs::TxsPath;

pub async fn route(
    State(state): State<AppState>,
    Path(path): Path<TxsPath>,
) -> ApiResult<TxsUtxosResponse> {
    let dolos = state.get_dolos()?;

    dolos.txs().utxos(&path.hash).await
}
