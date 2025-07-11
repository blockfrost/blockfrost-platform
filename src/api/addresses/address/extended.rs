use crate::{BlockfrostError, api::ApiResult};
use axum::{Extension, extract::Path};
use blockfrost_openapi::models::address_content_extended::AddressContentExtended;
use common::{
    addresses::{AddressInfo, AddressesPath},
    config::Config,
};

pub async fn route(
    Path(address_path): Path<AddressesPath>,
    Extension(config): Extension<Config>,
) -> ApiResult<AddressContentExtended> {
    let AddressesPath { address, asset: _ } = address_path;
    let _ = AddressInfo::from_address(&address, config.network)?;

    Err(BlockfrostError::not_found())
}
