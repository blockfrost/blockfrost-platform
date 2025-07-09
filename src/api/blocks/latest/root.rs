use crate::api::ApiResult;
use axum::{Extension, Json};
use blockfrost_openapi::models::block_content::BlockContent;
use dolos::client::Dolos;

pub async fn route(Extension(dolos): Extension<Dolos>) -> ApiResult<BlockContent> {
    let response: Json<BlockContent> = dolos.blocks_latest().await?;

    Ok(response)
}
