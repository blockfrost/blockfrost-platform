use crate::{
    BlockfrostError, api::utils::txs::evaluate::model::EvaluateQuery, server::state::AppState,
};
use axum::{
    Extension, Json,
    extract::{self, Query, State},
    response::IntoResponse,
};
use common::{
    config::Evaluator, helpers::binary_or_hex_heuristic, validation::validate_content_type,
};
use hyper::HeaderMap;
use node::pool::NodePool;
use tx_evaluator::{
    external::ExternalEvaluator,
    model::{TxEvaluationRequest, convert_eval_report_v5},
    native::evaluate_binary_tx,
};

pub async fn route(
    State(app_state): State<AppState>,
    Extension(node): Extension<NodePool>,
    Extension(fallback_evaluator): Extension<ExternalEvaluator>,
    headers: HeaderMap,
    Query(query): Query<EvaluateQuery>,
    extract::Json(tx_request): extract::Json<TxEvaluationRequest>,
) -> Result<impl IntoResponse, BlockfrostError> {
    // Allow only application/json content type
    validate_content_type(&headers, &["application/json"])?;

    let version: u8 = query.version.parse().unwrap();
    // query param overrides the config
    let is_external_evaluator = match query.evaluator {
        Some(v) => Evaluator::try_from(v)? == Evaluator::External,
        None => app_state.config.evaluator == Evaluator::External,
    };

    // safeguarding version and input data conflicts
    match tx_request {
        TxEvaluationRequest::V6(request) => {
            if version != 6 {
                Err(BlockfrostError::conflicting_ogmios_version())
            } else {
                let tx_cbor = binary_or_hex_heuristic(request.transaction.cbor.as_bytes());

                let result = if is_external_evaluator {
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
                let result = if is_external_evaluator {
                    fallback_evaluator
                        .evaluate_binary_tx_v5(
                            node,
                            tx_cbor.as_slice(),
                            request.additional_utxo_set,
                        )
                        .await?
                } else {
                    let r = convert_eval_report_v5(
                        evaluate_binary_tx(node, tx_cbor.as_slice(), request.additional_utxo_set)
                            .await?,
                    );
                    serde_json::to_value(r).unwrap()
                };
                Ok(Json(result))
            }
        },
        TxEvaluationRequest::V5Evaluate(request) => {
            if version != 5 {
                Err(BlockfrostError::conflicting_ogmios_version())
            } else {
                let tx_cbor = binary_or_hex_heuristic(request.evaluate.as_bytes());
                let result = if is_external_evaluator {
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
