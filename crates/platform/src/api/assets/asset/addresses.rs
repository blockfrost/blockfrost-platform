use crate::assets::{AssetData, AssetsPath};
use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, Query, State};
use bf_api_provider::types::AssetsAddressesResponse;
use bf_common::pagination::{Pagination, PaginationQuery};

pub async fn route(
    State(state): State<AppState>,
    Path(path): Path<AssetsPath>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<AssetsAddressesResponse> {
    let asset_data = AssetData::from_query(path.asset)?;
    let pagination = Pagination::from_query(pagination_query)?;
    let data_node = state.data_node()?;

    data_node
        .assets()
        .addresses(&asset_data.asset, &pagination)
        .await
}
