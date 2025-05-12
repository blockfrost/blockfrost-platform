use crate::{
    BlockfrostError,
    addresses::{AddressInfo, AddressesPath},
    api::ApiResult,
    config::Config,
};
use axum::{Extension, extract::Path};
use blockfrost_openapi::models::address_content_total::AddressContentTotal;

pub async fn route(
    Path(address_path): Path<AddressesPath>,
    Extension(config): Extension<Config>,
) -> ApiResult<AddressContentTotal> {
    let AddressesPath { address, asset: _ } = address_path;
    let _ = AddressInfo::from_address(&address, config.network)?;

    Err(BlockfrostError::not_found())
}
