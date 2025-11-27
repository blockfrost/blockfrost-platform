use crate::{api::ApiResult, server::state::AppState};
use axum::extract::State;
use bf_api_provider::types::NetworkErasResponse;

pub async fn route(State(state): State<AppState>) -> ApiResult<NetworkErasResponse> {
    let dolos = state.get_dolos()?;

    dolos.network().eras().await
}
