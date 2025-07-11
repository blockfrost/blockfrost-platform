use crate::api::ApiResult;
use axum::{Extension, extract::Path};
use blockfrost_openapi::models::block_content::BlockContent;
use common::blocks::BlocksSlotPath;
use dolos::client::Dolos;

pub async fn route(
    Extension(dolos): Extension<Dolos>,
    Path(blocks_slot_path): Path<BlocksSlotPath>,
) -> ApiResult<BlockContent> {
    let response = dolos.blocks_slot_slot(&blocks_slot_path.slot).await?;

    Ok(response)
}
