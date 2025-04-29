use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::block_content::BlockContent;

pub async fn route() -> ApiResult<BlockContent> {
    Err(BlockfrostError::not_found())
}
