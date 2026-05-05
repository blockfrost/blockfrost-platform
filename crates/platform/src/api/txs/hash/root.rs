use crate::txs::TxsPath;
use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, State};
use bf_api_provider::types::TxsSingleResponse;

pub async fn route(
    State(state): State<AppState>,
    Path(path): Path<TxsPath>,
) -> ApiResult<TxsSingleResponse> {
    let data_node = state.data_node()?;

    data_node.txs().by_hash(&path.hash).await
}
