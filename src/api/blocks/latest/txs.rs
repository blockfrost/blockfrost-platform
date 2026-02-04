use crate::{api::ApiResult, server::state::AppState};
use axum::extract::State;

pub async fn route(State(state): State<AppState>) -> ApiResult<Vec<String>> {
    let data_node = state.data_node()?;

    data_node.blocks().latest_txs().await
}
