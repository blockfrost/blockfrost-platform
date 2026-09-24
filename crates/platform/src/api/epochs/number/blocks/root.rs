use crate::api::ApiResult;
use crate::epochs::{EpochData, EpochsPath};
use crate::server::state::AppState;
use axum::extract::{Path, Query, State};
use bf_common::pagination::{Pagination, PaginationQuery};

pub async fn route(
    State(state): State<AppState>,
    Path(epochs_path): Path<EpochsPath>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<Vec<String>> {
    let epoch_data = EpochData::from_path(
        epochs_path.epoch_number,
        &state.config.network,
        &state.config.genesis,
    )?;
    let pagination = Pagination::from_query(pagination_query)?;
    let data_node = state.data_node()?;

    data_node
        .epochs()
        .blocks(&epoch_data.epoch_number, &pagination)
        .await
}
