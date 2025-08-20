use crate::server::state::AppState;
use api_provider::types::EpochsParamResponse;
use axum::extract::State;
use common::types::ApiResult;

pub async fn route(State(state): State<AppState>) -> ApiResult<EpochsParamResponse> {
    let dolos = state.get_dolos()?;

    dolos.epochs().latest_parameters().await
}
