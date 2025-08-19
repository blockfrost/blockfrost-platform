use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::TxsCborResponse;
use axum::extract::{Path, State};
use common::txs::TxsPath;

pub async fn route(
    State(state): State<AppState>,
    Path(path): Path<TxsPath>,
) -> ApiResult<TxsCborResponse> {
    state.dolos.txs().cbor(&path.hash).await
}
