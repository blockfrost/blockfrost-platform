use crate::{api::ApiResult, server::state::AppState};
use axum::extract::State;

pub async fn route(State(state): State<AppState>) -> ApiResult<Vec<String>> {
    state.api.dolos.blocks_latest_txs().await
}
