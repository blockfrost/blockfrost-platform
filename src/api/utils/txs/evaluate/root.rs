use crate::{BlockfrostError, api::utils::txs::evaluate::model::EvaluateQuery};
use axum::{Extension, Json, extract::Query, response::IntoResponse};
use common::{helpers::binary_or_hex_heuristic, validation::validate_content_type};
use hyper::HeaderMap;
use node::pool::NodePool;
use tx_evaluator::external::ExternalEvaluator;

pub async fn route(
    Extension(node): Extension<NodePool>,
    Extension(fallback_evaluator): Extension<ExternalEvaluator>,
    Query(query): Query<EvaluateQuery>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<impl IntoResponse, BlockfrostError> {
    // Allow only application/cbor content type
    validate_content_type(&headers, &["application/cbor"])?;

    // Allow both hex-encoded and raw binary bodies
    let tx_cbor_binary = binary_or_hex_heuristic(body.as_ref());

    let result = fallback_evaluator
        .evaluate_binary_tx_v5(node, tx_cbor_binary.as_slice(), None)
        .await?;

    Ok(Json(result))
}
