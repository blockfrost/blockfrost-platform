use crate::{api::ApiResult, server::state::AppState};
use axum::extract::State;
use bf_api_provider::types::NetworkErasResponse;

pub async fn route(State(state): State<AppState>) -> ApiResult<NetworkErasResponse> {
    let data_node = state.data_node()?;

    data_node.network().eras().await
}
