use crate::{api::ApiResult, server::state::AppState};
use axum::extract::State;
use bf_api_provider::types::NetworkResponse;

pub async fn route(State(state): State<AppState>) -> ApiResult<NetworkResponse> {
    let dolos = state.get_dolos()?;

    dolos.network().get().await
}
