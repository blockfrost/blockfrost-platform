use crate::{
    BlockfrostError, NodePool,
    cbor::evaluate,
    common::{binary_or_hex_heuristic, validate_content_type},
};
use axum::{Extension, Json, response::IntoResponse};
use hyper::HeaderMap;

use super::model::convert_eval_report;

pub async fn route(
    Extension(node): Extension<NodePool>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<impl IntoResponse, BlockfrostError> {
    // Allow only application/cbor content type
    validate_content_type(&headers, &["application/cbor"])?;

    // Allow both hex-encoded and raw binary bodies
    let binary_tx = binary_or_hex_heuristic(body.as_ref());

    let report = evaluate::evaluate_binary_tx(node, &binary_tx, None).await?;
    let result = convert_eval_report(report);

    Ok(Json(result))
}
