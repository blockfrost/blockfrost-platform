use crate::api::ApiResult;
use axum::Extension;
use blockfrost_openapi::models::network_eras_inner::NetworkErasInner;
use dolos::client::Dolos;

pub async fn route(Extension(dolos): Extension<Dolos>) -> ApiResult<Vec<NetworkErasInner>> {
    let response = dolos.network_eras().await?;

    Ok(response)
}
