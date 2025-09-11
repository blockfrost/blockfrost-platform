use crate::server::state::AppState;
use api_provider::types::EpochsParamResponse;
use axum::extract::{Path, State};
use common::{
    epochs::{EpochData, EpochsPath},
    types::ApiResult,
};

pub async fn route(
    State(state): State<AppState>,
    Path(epochs_path): Path<EpochsPath>,
) -> ApiResult<EpochsParamResponse> {
    let epoch_data = EpochData::from_path(epochs_path.epoch_number, &state.config.network)?;
    let dolos = state.get_dolos()?;

    dolos.epochs().parameters(&epoch_data.epoch_number).await
}
