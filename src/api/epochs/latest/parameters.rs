use crate::server::state::AppState;
use axum::extract::State;
use bf_api_provider::types::EpochsParamResponse;
use bf_common::types::ApiResult;

pub async fn route(State(state): State<AppState>) -> ApiResult<EpochsParamResponse> {
    let data_node = state.data_node()?;

    data_node.epochs().latest_parameters().await
}
