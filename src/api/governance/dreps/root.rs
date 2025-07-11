use axum::{Extension, extract::Path};
use blockfrost_openapi::models::drep::Drep;
use common::{dreps::DrepsPath, types::ApiResult};
use dolos::client::Dolos;

pub async fn route(
    Path(drep_path): Path<DrepsPath>,
    Extension(dolos): Extension<Dolos>,
) -> ApiResult<Drep> {
    dolos.governance_dreps_drep_id(&drep_path.drep_id).await
}
