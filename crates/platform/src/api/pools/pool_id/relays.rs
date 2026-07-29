use crate::{
    api::ApiResult,
    pools::{PoolData, PoolsPath},
    server::state::AppState,
};
use axum::extract::{Path, State};
use bf_api_provider::types::PoolsRelaysResponse;

pub async fn route(
    State(state): State<AppState>,
    Path(pools_path): Path<PoolsPath>,
) -> ApiResult<PoolsRelaysResponse> {
    let pool_data = PoolData::from_path(&pools_path.pool_id)?;
    let data_node = state.data_node()?;

    data_node.pools().relays(&pool_data.pool_id).await
}
