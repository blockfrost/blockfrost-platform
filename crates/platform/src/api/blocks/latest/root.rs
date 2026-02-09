use crate::{api::ApiResult, server::state::AppState};
use axum::extract::State;
use bf_api_provider::types::BlocksSingleResponse;

pub async fn route(State(state): State<AppState>) -> ApiResult<BlocksSingleResponse> {
    let dolos = state.get_dolos()?;

    dolos.blocks().latest().await
}
