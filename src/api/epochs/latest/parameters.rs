use crate::server::state::AppState;
use axum::extract::State;
use bf_api_provider::types::EpochsParamResponse;
use bf_common::types::ApiResult;

pub async fn route(State(state): State<AppState>) -> ApiResult<EpochsParamResponse> {
    let dolos = state.get_dolos()?;

    dolos.epochs().latest_parameters().await
}
