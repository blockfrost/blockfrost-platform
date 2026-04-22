use crate::epochs::{EpochData, EpochsPath};
use crate::server::state::AppState;
use axum::extract::{Path, State};
use bf_api_provider::types::EpochsParamResponse;
use bf_common::types::ApiResult;

pub async fn route(
    State(state): State<AppState>,
    Path(epochs_path): Path<EpochsPath>,
) -> ApiResult<EpochsParamResponse> {
    let epoch_data = EpochData::from_path(epochs_path.epoch_number, &state.config.network)?;
    let data_node = state.data_node()?;

    data_node
        .epochs()
        .parameters(&epoch_data.epoch_number)
        .await
}
