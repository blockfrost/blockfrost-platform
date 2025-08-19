use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::PoolsListExtendedResponse;
use axum::extract::State;

pub async fn route(State(state): State<AppState>) -> ApiResult<PoolsListExtendedResponse> {
    state.dolos.pools().extended().await
}
