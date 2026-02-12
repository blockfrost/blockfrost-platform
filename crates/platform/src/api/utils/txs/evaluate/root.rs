use crate::{
    BlockfrostError, api::utils::txs::evaluate::model::EvaluateQuery, server::state::AppState,
};
use axum::{
    Extension, Json,
    extract::{Query, State},
    response::IntoResponse,
};
use bf_common::{helpers::binary_or_hex_heuristic, validation::validate_content_type};
use bf_node::pool::NodePool;
use bf_tx_evaluator::{
    external::ExternalEvaluator, helpers::is_external_evaluator,
    model::api::convert_eval_report_v5, native::evaluate_binary_tx,
    wrapper::wrap_success_response_v5,
};
use hyper::HeaderMap;

pub async fn route(
    State(app_state): State<AppState>,
    Extension(node): Extension<NodePool>,
    Extension(fallback_evaluator_opt): Extension<Option<ExternalEvaluator>>,
    Query(query): Query<EvaluateQuery>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<impl IntoResponse, BlockfrostError> {
    // Allow only application/cbor content type
    validate_content_type(&headers, &["application/cbor"])?;

    // Allow both hex-encoded and raw binary bodies
    let tx_cbor_binary = binary_or_hex_heuristic(body.as_ref());

    let is_external_evaluator = is_external_evaluator(
        query.evaluator,
        &app_state.config.evaluator,
        &fallback_evaluator_opt,
    )?;

    match query.version.parse().unwrap() {
        5 => {
            if is_external_evaluator {
                Ok(Json(
                    fallback_evaluator_opt
                        .unwrap()
                        .evaluate_binary_tx_v5(node, tx_cbor_binary.as_slice(), None)
                        .await?,
                ))
            } else {
                let r = convert_eval_report_v5(
                    evaluate_binary_tx(node, tx_cbor_binary.as_slice(), None).await?,
                );
                Ok(Json(wrap_success_response_v5(r, serde_json::Value::Null)))
            }
        },

        6 => {
            if is_external_evaluator {
                Ok(Json(
                    fallback_evaluator_opt
                        .unwrap()
                        .evaluate_binary_tx_v6(node, tx_cbor_binary.as_slice(), None)
                        .await?,
                ))
            } else {
                todo!("native evaluator for v6 not implemented yet")
            }
        },
        version => Err(BlockfrostError::custom_400(format!(
            "invalid version {version}"
        ))),
    }
}
