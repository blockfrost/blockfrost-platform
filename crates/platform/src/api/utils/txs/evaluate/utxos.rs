use crate::BlockfrostError;
use axum::{
    Extension, Json,
    extract::{self},
    response::IntoResponse,
};
use bf_node::pool::NodePool;
use bf_tx_evaluator::{external::ExternalEvaluator, model::api::TxEvaluationRequest};

pub async fn route(
    Extension(node): Extension<NodePool>,
    Extension(evaluator): Extension<ExternalEvaluator>,
    extract::Json(tx_request): extract::Json<TxEvaluationRequest>,
) -> Result<impl IntoResponse, BlockfrostError> {
    // query.version is ignored on purpose
    match tx_request {
        TxEvaluationRequest::V6(request) => Ok(Json(
            evaluator
                .evaluate_tx_payload_v6(
                    node,
                    request.transaction.cbor.as_bytes(),
                    request.additional_utxo,
                )
                .await?,
        )),
        TxEvaluationRequest::V5Cbor(request) => Ok(Json(
            evaluator
                .evaluate_tx_payload_v5(node, request.cbor.as_bytes(), request.additional_utxo_set)
                .await?,
        )),
        TxEvaluationRequest::V5Evaluate(request) => Ok(Json(
            evaluator
                .evaluate_tx_payload_v5(
                    node,
                    request.evaluate.as_bytes(),
                    request.additional_utxo_set,
                )
                .await?,
        )),
    }
}
