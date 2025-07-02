use crate::{BlockfrostError, api::ApiResult, config::Config};
use axum::{Extension, extract::Path};
use common::addresses::{AddressInfo, AddressesPath};

pub async fn route(
    Path(address_path): Path<AddressesPath>,
    Extension(config): Extension<Config>,
) -> ApiResult<()> {
    let AddressesPath { address, asset: _ } = address_path;
    let _ = AddressInfo::from_address(&address, config.network)?;

    Err(BlockfrostError::not_found())
}
