use crate::{api::ApiResult, cli::Config, genesis::get_genesis_content_for};

use axum::{Extension, Json};
use blockfrost_openapi::models::genesis_content::GenesisContent;

pub async fn route(Extension(config): Extension<Config>) -> ApiResult<GenesisContent> {
    let genesis = get_genesis_content_for(&config.network);

    Ok(Json(genesis))
}
