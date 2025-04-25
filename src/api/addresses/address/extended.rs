use crate::{BlockfrostError, api::ApiResult};

use blockfrost_openapi::models::address_content_extended::AddressContentExtended;

pub async fn route() -> ApiResult<AddressContentExtended> {
    Err(BlockfrostError::not_found())
}
