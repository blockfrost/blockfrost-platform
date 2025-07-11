use axum::{Extension, extract::Path};
use blockfrost_openapi::models::epoch_param_content::EpochParamContent;
use common::{epochs::EpochsPath, types::ApiResult};
use dolos::client::Dolos;

pub async fn route(
    Path(epochs_path): Path<EpochsPath>,
    Extension(dolos): Extension<Dolos>,
) -> ApiResult<EpochParamContent> {
    dolos
        .epoch_number_parameters(&epochs_path.epoch_number)
        .await
}
