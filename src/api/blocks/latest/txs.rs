use crate::api::ApiResult;
use axum::{Extension, Json};
use blockfrost_openapi::models::tx_content::TxContent;
use dolos::client::Dolos;

pub async fn route(Extension(dolos): Extension<Dolos>) -> ApiResult<Vec<TxContent>> {
    let response: Json<Vec<TxContent>> = dolos.blocks_latest_txs().await?;

    Ok(response)
}
