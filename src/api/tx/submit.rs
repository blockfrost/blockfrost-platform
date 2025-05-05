use crate::{
    BlockfrostError, NodePool,
    common::{binary_or_hex_heuristic, validate_content_type},
};
use axum::{Extension, Json, http::HeaderMap, response::IntoResponse};
use metrics::counter;

pub async fn route(
    Extension(node): Extension<NodePool>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<impl IntoResponse, BlockfrostError> {
    // Allow only application/cbor content type
    validate_content_type(&headers, &["application/cbor"])?;

    // Allow both hex-encoded and raw binary bodies
    let binary_tx = binary_or_hex_heuristic(body.as_ref());

    // XXX: Axum must not abort Ouroboros protocols in the middle, hence a separate Tokio task:
    let response_body = tokio::spawn(async move {
        // Submit transaction
        let mut node = node.get().await?;
        let response = node.submit_transaction(binary_tx).await;

        if response.is_ok() {
            counter!("tx_submit_success").increment(1)
        } else {
            counter!("tx_submit_failure").increment(1)
        }

        response
    })
    .await
    .expect("submit_transaction panic!")?;

    let mut response_headers = HeaderMap::new();

    response_headers.insert(
        "blockfrost-platform-response",
        response_body.to_string().parse()?,
    );

    Ok((response_headers, Json(response_body)))
}
