use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::block_content_addresses_inner::BlockContentAddressesInner;

pub async fn route() -> ApiResult<Vec<BlockContentAddressesInner>> {
    Err(BlockfrostError::not_found())
}
