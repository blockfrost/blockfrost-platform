use crate::{
    api::ApiResult,
    cli::Config,
    genesis::{GenesisRegistry, genesis},
};

use axum::{Extension, Json};
use blockfrost_openapi::models::genesis_content::GenesisContent;

pub async fn route(Extension(config): Extension<Config>) -> ApiResult<GenesisContent> {
    let genesis = genesis().by_network(&config.network);

    Ok(Json(genesis.clone()))
}
