use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Path, State};
use bf_api_provider::types::PoolsSingleResponse;
use bf_common::pools::{PoolData, PoolsPath};

pub async fn route(
    State(state): State<AppState>,
    Path(pools_path): Path<PoolsPath>,
) -> ApiResult<PoolsSingleResponse> {
    let pool_data = PoolData::from_path(&pools_path.pool_id)?;
    let data_node = state.data_node()?;

    data_node.pools().by_id(&pool_data.pool_id).await
}
