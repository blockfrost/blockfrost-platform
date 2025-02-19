use crate::{common::validate_content_type, BlockfrostError, NodePool};
use axum::{http::HeaderMap, response::IntoResponse, Extension, Json};
use metrics::gauge;

pub async fn route(
    Extension(node): Extension<NodePool>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<impl IntoResponse, BlockfrostError> {
    // Allow only application/cbor content type
    validate_content_type(&headers, &["application/cbor"])?;

    // XXX: Axum must not abort Ouroboros protocols in the middle, hence a separate Tokio task:
    let response = tokio::spawn(async move {
        // Submit transaction
        let mut node = node.get().await?;
        let response = node.submit_transaction(body.as_ref()).await;

        if response.is_ok() {
            gauge!("tx_submit_success").increment(1)
        } else {
            gauge!("tx_submit_failure").increment(1)
        }

        response
    })
    .await
    .expect("submit_transaction panic!")?;

    Ok(Json(response))
}
