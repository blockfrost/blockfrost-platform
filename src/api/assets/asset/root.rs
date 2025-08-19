use crate::{api::ApiResult, server::state::AppState};
use api_provider::types::AssetsSingleResponse;
use axum::extract::{Path, State};
use common::assets::{AssetData, AssetsPath};

pub async fn route(
    State(state): State<AppState>,
    Path(path): Path<AssetsPath>,
) -> ApiResult<AssetsSingleResponse> {
    let asset_data = AssetData::from_query(path.asset)?;

    state.dolos.assets().asset(&asset_data.asset).await
}
