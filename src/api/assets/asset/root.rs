use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, State};
use blockfrost_openapi::models::asset::Asset;
use common::assets::{AssetData, AssetsPath};

pub async fn route(
    State(state): State<AppState>,
    Path(path): Path<AssetsPath>,
) -> ApiResult<Asset> {
    let asset_data = AssetData::from_query(path.asset)?;

    state.api.dolos.assets_asset(&asset_data.asset).await
}
