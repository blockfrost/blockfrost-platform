use crate::{BlockfrostError, api::ApiResult};

use blockfrost_openapi::models::network::Network;

pub async fn route() -> ApiResult<Network> {
    Err(BlockfrostError::not_found())
}
