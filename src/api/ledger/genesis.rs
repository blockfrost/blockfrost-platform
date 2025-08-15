use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::GenesisResponse;
use axum::{Json, extract::State};
use common::genesis::GenesisRegistry;

pub async fn route(State(state): State<AppState>) -> ApiResult<GenesisResponse> {
    let genesis = state.genesis.by_network(&state.config.network);

    Ok(Json(genesis.clone()))
}
