use crate::server::state::AppState;
use api_provider::types::EpochsParamResponse;
use axum::extract::{Path, State};
use common::{epochs::EpochsPath, types::ApiResult};

pub async fn route(
    State(state): State<AppState>,
    Path(epochs_path): Path<EpochsPath>,
) -> ApiResult<EpochsParamResponse> {
    state
        .dolos
        .epochs()
        .parameters(&epochs_path.epoch_number)
        .await
}
