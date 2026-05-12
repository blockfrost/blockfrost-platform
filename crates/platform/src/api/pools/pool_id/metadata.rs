use crate::{
    api::ApiResult,
    pools::{PoolData, PoolsPath},
    server::state::AppState,
};
use axum::extract::{Path, State};
use bf_api_provider::types::PoolsMetadataResponse;

pub async fn route(
    State(state): State<AppState>,
    Path(pools_path): Path<PoolsPath>,
) -> ApiResult<PoolsMetadataResponse> {
    let pool_data = PoolData::from_path(&pools_path.pool_id)?;
    let data_node = state.data_node()?;

    data_node.pools().metadata(&pool_data.pool_id).await
}
