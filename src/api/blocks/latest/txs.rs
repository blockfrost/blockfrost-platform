use crate::{api::ApiResult, server::state::AppState};
use axum::extract::State;

pub async fn route(State(state): State<AppState>) -> ApiResult<Vec<String>> {
    state.dolos.blocks().latest_txs().await
}
