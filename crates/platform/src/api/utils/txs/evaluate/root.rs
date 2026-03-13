use crate::{BlockfrostError, api::utils::txs::evaluate::model::EvaluateQuery};
use axum::{Extension, Json, extract::Query, response::IntoResponse};
use bf_common::{helpers::binary_or_hex_heuristic, validation::validate_content_type};
use bf_node::pool::NodePool;
use bf_tx_evaluator::external::ExternalEvaluator;
use hyper::HeaderMap;

pub async fn route(
    Extension(node): Extension<NodePool>,
    Extension(evaluator): Extension<ExternalEvaluator>,
    Query(query): Query<EvaluateQuery>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<impl IntoResponse, BlockfrostError> {
    // Allow only application/cbor content type
    validate_content_type(&headers, &["application/cbor"])?;

    // Allow both hex-encoded and raw binary bodies
    let tx_cbor_binary = binary_or_hex_heuristic(body.as_ref());

    match query.version {
        5 => Ok(Json(
            evaluator
                .evaluate_binary_tx_v5(node, tx_cbor_binary.as_slice(), None)
                .await?,
        )),
        6 => Ok(Json(
            evaluator
                .evaluate_binary_tx_v6(node, tx_cbor_binary.as_slice(), None)
                .await?,
        )),
        version => Err(BlockfrostError::custom_400(format!(
            "invalid version {version}"
        ))),
    }
}
