use crate::{api::ApiResult, server::state::AppState};
use axum::extract::State;
use blockfrost_openapi::models::pool_list_extended_inner::PoolListExtendedInner;

pub async fn route(State(state): State<AppState>) -> ApiResult<Vec<PoolListExtendedInner>> {
    state.api.dolos.pools_extended().await
}
