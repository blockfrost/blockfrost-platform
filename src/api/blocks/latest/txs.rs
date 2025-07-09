use crate::api::ApiResult;
use axum::{Extension, Json};
use dolos::client::Dolos;

pub async fn route(Extension(dolos): Extension<Dolos>) -> ApiResult<Vec<String>> {
    let response: Json<Vec<String>> = dolos.blocks_latest_txs().await?;

    Ok(response)
}
