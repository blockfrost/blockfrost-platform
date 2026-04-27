use crate::assets::{AssetData, AssetsPath};
use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, State};
use bf_api_provider::types::AssetsSingleResponse;

pub async fn route(
    State(state): State<AppState>,
    Path(path): Path<AssetsPath>,
) -> ApiResult<AssetsSingleResponse> {
    let asset_data = AssetData::from_query(path.asset)?;
    let data_node = state.data_node()?;

    data_node.assets().asset(&asset_data.asset).await
}
