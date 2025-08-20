use api_provider::types::BlocksAddressesContentResponse;

use crate::{BlockfrostError, api::ApiResult};

pub async fn route() -> ApiResult<BlocksAddressesContentResponse> {
    Err(BlockfrostError::not_found())
}
