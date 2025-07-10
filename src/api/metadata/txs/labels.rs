use crate::api::ApiResult;
use axum::Extension;
use blockfrost_openapi::models::tx_metadata_labels_inner::TxMetadataLabelsInner;
use dolos::client::Dolos;

pub async fn route(Extension(dolos): Extension<Dolos>) -> ApiResult<Vec<TxMetadataLabelsInner>> {
    dolos.metadata_txs_labels().await
}
