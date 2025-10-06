use crate::{BlockfrostError, api::utils::txs::evaluate::model::EvaluateQuery};
use axum::{
    Extension, Json,
    extract::{self, Query},
    response::IntoResponse,
};
use common::{helpers::binary_or_hex_heuristic, validation::validate_content_type};
use hyper::HeaderMap;
use node::pool::NodePool;
use tx_evaluator::{external::ExternalEvaluator, model::TxEvaluationRequest};

pub async fn route(
    Extension(node): Extension<NodePool>,
    Extension(fallback_evaluator): Extension<ExternalEvaluator>,
    headers: HeaderMap,
    Query(query): Query<EvaluateQuery>,
    extract::Json(tx_request): extract::Json<TxEvaluationRequest>,
) -> Result<impl IntoResponse, BlockfrostError> {
    // Allow only application/json content type
    validate_content_type(&headers, &["application/json"])?;

    let version: u8 = query.version.parse().unwrap();

    // safeguarding version and input data conflicts
    match tx_request {
        TxEvaluationRequest::V6(request) => {
            if version != 6 {
                Err(BlockfrostError::conflicting_ogmios_version())
            } else {
                let tx_cbor = binary_or_hex_heuristic(request.transaction.cbor.as_bytes());
                let result = fallback_evaluator
                    .evaluate_binary_tx_v6(node, tx_cbor.as_slice(), request.additional_utxo)
                    .await?;
                Ok(Json(result))
            }
        },
        TxEvaluationRequest::V5Cbor(request) => {
            if version != 5 {
                Err(BlockfrostError::conflicting_ogmios_version())
            } else {
                let tx_cbor = binary_or_hex_heuristic(request.cbor.as_bytes());
                let result = fallback_evaluator
                    .evaluate_binary_tx_v5(node, tx_cbor.as_slice(), request.additional_utxo_set)
                    .await?;
                Ok(Json(result))
            }
        },
        TxEvaluationRequest::V5Evaluate(request) => {
            if version != 5 {
                Err(BlockfrostError::conflicting_ogmios_version())
            } else {
                let tx_cbor = binary_or_hex_heuristic(request.evaluate.as_bytes());
                let result = fallback_evaluator
                    .evaluate_binary_tx_v5(node, tx_cbor.as_slice(), request.additional_utxo_set)
                    .await?;
                Ok(Json(result))
            }
        },
    }
}
