use crate::{api::ApiResult, server::state::AppState};
use axum::{Json, extract::State};
use blockfrost_openapi::models::genesis_content::GenesisContent;
use common::genesis::GenesisRegistry;

pub async fn route(State(state): State<AppState>) -> ApiResult<GenesisContent> {
    let genesis = state.genesis.by_network(&state.config.network);

    Ok(Json(genesis.clone()))
}
