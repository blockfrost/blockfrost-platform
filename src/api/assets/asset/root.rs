use crate::api::ApiResult;
use axum::{Extension, extract::Path};
use blockfrost_openapi::models::asset::Asset;
use common::assets::{AssetData, AssetsPath};
use dolos::client::Dolos;

pub async fn route(
    Path(path): Path<AssetsPath>,
    Extension(dolos): Extension<Dolos>,
) -> ApiResult<Asset> {
    let asset_data = AssetData::from_query(path.asset)?;

    dolos.assets_asset(&asset_data.asset).await
}
