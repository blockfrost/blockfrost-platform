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
    // @todo read from config if not provided
    let evaluator = query.evaluator.unwrap_or("external".to_string());

    // safeguarding version and input data conflicts
    match tx_request {
        TxEvaluationRequest::V6(request) => {
            if version != 6 {
                Err(BlockfrostError::conflicting_ogmios_version())
            } else {
                let tx_cbor = binary_or_hex_heuristic(request.transaction.cbor.as_bytes());

                let result = if evaluator == "external" {
                    fallback_evaluator
                        .evaluate_binary_tx_v6(node, tx_cbor.as_slice(), request.additional_utxo)
                        .await?
                } else {
                    todo!("native evaluator for v6 not implemented yet")
                };
                Ok(Json(result))
            }
        },
        TxEvaluationRequest::V5Cbor(request) => {
            if version != 5 {
                Err(BlockfrostError::conflicting_ogmios_version())
            } else {
                let tx_cbor = binary_or_hex_heuristic(request.cbor.as_bytes());
                let result = if evaluator == "external" {
                    fallback_evaluator
                        .evaluate_binary_tx_v5(
                            node,
                            tx_cbor.as_slice(),
                            request.additional_utxo_set,
                        )
                        .await?
                } else {
                    // evaluate_binary_tx(node, tx_cbor, request.additional_utxo_set).await?
                    todo!("native evaluator for v5 not implemented yet")
                };
                Ok(Json(result))
            }
        },
        TxEvaluationRequest::V5Evaluate(request) => {
            if version != 5 {
                Err(BlockfrostError::conflicting_ogmios_version())
            } else {
                let tx_cbor = binary_or_hex_heuristic(request.evaluate.as_bytes());
                let result = if evaluator == "external" {
                    fallback_evaluator
                        .evaluate_binary_tx_v5(
                            node,
                            tx_cbor.as_slice(),
                            request.additional_utxo_set,
                        )
                        .await?
                } else {
                    todo!("native evaluator for v5 not implemented yet")
                };
                Ok(Json(result))
            }
        },
    }
}
