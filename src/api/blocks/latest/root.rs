use crate::api::ApiResult;
use axum::Extension;
use blockfrost_openapi::models::block_content::BlockContent;
use dolos::client::Dolos;

pub async fn route(Extension(dolos): Extension<Dolos>) -> ApiResult<BlockContent> {
    dolos.blocks_latest().await
}
