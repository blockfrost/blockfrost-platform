use crate::{api::ApiResult, server::state::AppState};
use axum::extract::State;
use blockfrost_openapi::models::network_eras_inner::NetworkErasInner;

pub async fn route(State(state): State<AppState>) -> ApiResult<Vec<NetworkErasInner>> {
    state.api.dolos.network_eras().await
}
