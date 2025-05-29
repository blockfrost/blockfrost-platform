use crate::{api::ApiResult, genesis::GenesisRegistry, server::state::AppState};
use axum::{Json, extract::State};
use blockfrost_openapi::models::genesis_content::GenesisContent;

pub async fn route(
    State(AppState { config, genesis }): State<AppState>,
) -> ApiResult<GenesisContent> {
    let genesis = genesis.by_network(&config.network);

    Ok(Json(genesis.clone()))
}
