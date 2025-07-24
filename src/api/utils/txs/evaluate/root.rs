use crate::{
    BlockfrostError, NodePool,
    cbor::external::fallback_evaluator::FallbackEvaluator,
    common::{binary_or_hex_heuristic, validate_content_type},
};
use axum::{Extension, Json, response::IntoResponse};
use hyper::HeaderMap;

pub async fn route(
    Extension(node): Extension<NodePool>,
    Extension(fallback_evaluator): Extension<FallbackEvaluator>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<impl IntoResponse, BlockfrostError> {
    // Allow only application/cbor content type
    validate_content_type(&headers, &["application/cbor"])?;

    // Allow both hex-encoded and raw binary bodies
    let tx_cbor_binary = binary_or_hex_heuristic(body.as_ref());

    let result = fallback_evaluator
        .evaluate_binary_tx(node, tx_cbor_binary.as_slice(), None)
        .await?;

    Ok(Json(result))
}
