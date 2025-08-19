use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::NetworkErasResponse;
use axum::extract::State;

pub async fn route(State(state): State<AppState>) -> ApiResult<NetworkErasResponse> {
    state.dolos.network().eras().await
}
