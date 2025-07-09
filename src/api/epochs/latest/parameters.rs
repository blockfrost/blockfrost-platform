use axum::Extension;
use blockfrost_openapi::models::epoch_param_content::EpochParamContent;
use common::types::ApiResult;
use dolos::client::Dolos;

pub async fn route(Extension(dolos): Extension<Dolos>) -> ApiResult<EpochParamContent> {
    dolos.epoch_latest_parameters().await
}
