use crate::{api::ApiResult, server::state::AppState};
use axum::extract::State;
use blockfrost_openapi::models::network::Network;

pub async fn route(State(state): State<AppState>) -> ApiResult<Network> {
    state.api.dolos.network().await
}
