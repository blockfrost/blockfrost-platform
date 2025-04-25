use crate::{BlockfrostError, api::ApiResult};
use blockfrost_openapi::models::network_eras_inner::NetworkErasInner;

pub async fn route() -> ApiResult<Vec<NetworkErasInner>> {
    Err(BlockfrostError::not_found())
}
