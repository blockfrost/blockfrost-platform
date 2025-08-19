use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::NetworkResponse;
use axum::extract::State;

pub async fn route(State(state): State<AppState>) -> ApiResult<NetworkResponse> {
    let dolos = state.get_dolos()?;

    dolos.network().get().await
}
