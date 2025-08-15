use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::BlocksSingleResponse;
use axum::extract::State;

pub async fn route(State(state): State<AppState>) -> ApiResult<BlocksSingleResponse> {
    state.api.dolos.blocks_latest().await
}
