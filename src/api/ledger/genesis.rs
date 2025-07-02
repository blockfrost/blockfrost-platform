use crate::{api::ApiResult, server::state::AppState};
use axum::{Json, extract::State};
use blockfrost_openapi::models::genesis_content::GenesisContent;
use common::genesis::GenesisRegistry;

pub async fn route(
    State(AppState { config, genesis }): State<AppState>,
) -> ApiResult<GenesisContent> {
    let genesis = genesis.by_network(&config.network);

    Ok(Json(genesis.clone()))
}
