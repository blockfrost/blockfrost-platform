use crate::{api::ApiResult, server::state::AppState};
use axum::extract::State;
use bf_api_provider::types::NetworkResponse;

pub async fn route(State(state): State<AppState>) -> ApiResult<NetworkResponse> {
    let data_node = state.data_node()?;

    data_node.network().get().await
}
