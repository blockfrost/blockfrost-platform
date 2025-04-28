use crate::{
    BlockfrostError,
    addresses::{AddressInfo, AddressesPath},
    api::ApiResult,
    cli::Config,
};
use axum::{Extension, extract::Path};

pub async fn route(
    Path(address_path): Path<AddressesPath>,
    Extension(config): Extension<Config>,
) -> ApiResult<()> {
    let AddressesPath { address, asset: _ } = address_path;
    let _ = AddressInfo::from_address(&address, config.network)?;

    Err(BlockfrostError::not_found())
}
