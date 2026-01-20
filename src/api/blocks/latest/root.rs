use crate::{api::ApiResult, server::state::AppState};
use axum::extract::State;
use bf_api_provider::types::BlocksSingleResponse;

pub async fn route(State(state): State<AppState>) -> ApiResult<BlocksSingleResponse> {
    let data_node = state.get_data_node()?;

    data_node.blocks().latest().await
}
