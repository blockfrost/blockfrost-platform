use crate::server::state::AppState;
use api_provider::types::EpochParamResponse;
use axum::extract::{Path, State};
use common::{epochs::EpochsPath, types::ApiResult};

pub async fn route(
    State(state): State<AppState>,
    Path(epochs_path): Path<EpochsPath>,
) -> ApiResult<EpochParamResponse> {
    state
        .api
        .dolos
        .epoch_number_parameters(&epochs_path.epoch_number)
        .await
}
