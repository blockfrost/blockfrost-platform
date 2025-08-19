use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::TxsSingleResponse;
use axum::extract::{Path, State};
use common::txs::TxsPath;

pub async fn route(
    State(state): State<AppState>,
    Path(path): Path<TxsPath>,
) -> ApiResult<TxsSingleResponse> {
    let dolos = state.get_dolos()?;

    dolos.txs().by_hash(&path.hash).await
}
