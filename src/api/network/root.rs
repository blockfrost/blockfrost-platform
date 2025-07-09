use crate::api::ApiResult;
use axum::Extension;
use blockfrost_openapi::models::network::Network;
use dolos::client::Dolos;

pub async fn route(Extension(dolos): Extension<Dolos>) -> ApiResult<Network> {
    let response = dolos.network().await?;

    Ok(response)
}
