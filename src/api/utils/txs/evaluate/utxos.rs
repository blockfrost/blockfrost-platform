use crate::{BlockfrostError, NodePool, cbor::evaluate, common::validate_content_type};
use axum::{Extension, Json, extract, response::IntoResponse};
use hyper::HeaderMap;

use super::model::{TxEvaluationRequest, convert_eval_report};

pub async fn route(
    Extension(node): Extension<NodePool>,
    headers: HeaderMap,
    extract::Json(tx_request): extract::Json<TxEvaluationRequest>,
) -> Result<impl IntoResponse, BlockfrostError> {
    // Allow only application/json content type
    validate_content_type(&headers, &["application/json"])?;

    let pallas_report =
        evaluate::evaluate_encoded_tx(node, &tx_request.cbor, tx_request.additional_utxo_set)
            .await?;

    Ok(Json(convert_eval_report(pallas_report)))
}
