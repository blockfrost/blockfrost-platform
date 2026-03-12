use crate::{BlockfrostError, api::utils::txs::evaluate::model::EvaluateQuery};
use axum::{
    Extension, Json,
    extract::{self, Query},
    response::IntoResponse,
};
use bf_common::helpers::binary_or_hex_heuristic;
use bf_node::pool::NodePool;
use bf_tx_evaluator::{external::ExternalEvaluator, model::api::TxEvaluationRequest};

pub async fn route(
    Extension(node): Extension<NodePool>,
    Extension(evaluator): Extension<ExternalEvaluator>,
    Query(query): Query<EvaluateQuery>,
    extract::Json(tx_request): extract::Json<TxEvaluationRequest>,
) -> Result<impl IntoResponse, BlockfrostError> {
    let version = query.version;

    // safeguarding version and input data conflicts
    match tx_request {
        TxEvaluationRequest::V6(request) => {
            if version != 6 {
                Err(BlockfrostError::conflicting_ogmios_version())
            } else {
                let tx_cbor = binary_or_hex_heuristic(request.transaction.cbor.as_bytes());
                Ok(Json(
                    evaluator
                        .evaluate_binary_tx_v6(node, tx_cbor.as_slice(), request.additional_utxo)
                        .await?,
                ))
            }
        },
        TxEvaluationRequest::V5Cbor(request) => {
            if version != 5 {
                Err(BlockfrostError::conflicting_ogmios_version())
            } else {
                let tx_cbor = binary_or_hex_heuristic(request.cbor.as_bytes());
                Ok(Json(
                    evaluator
                        .evaluate_binary_tx_v5(
                            node,
                            tx_cbor.as_slice(),
                            request.additional_utxo_set,
                        )
                        .await?,
                ))
            }
        },
        TxEvaluationRequest::V5Evaluate(request) => {
            if version != 5 {
                Err(BlockfrostError::conflicting_ogmios_version())
            } else {
                let tx_cbor = binary_or_hex_heuristic(request.evaluate.as_bytes());
                Ok(Json(
                    evaluator
                        .evaluate_binary_tx_v5(
                            node,
                            tx_cbor.as_slice(),
                            request.additional_utxo_set,
                        )
                        .await?,
                ))
            }
        },
    }
}
