use crate::{api::ApiResult, server::state::AppState};
use axum::extract::State;
use blockfrost_openapi::models::block_content::BlockContent;

pub async fn route(State(state): State<AppState>) -> ApiResult<BlockContent> {
    state.api.dolos.blocks_latest().await
}
