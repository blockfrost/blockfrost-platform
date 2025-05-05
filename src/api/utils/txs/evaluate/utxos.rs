use crate::{BlockfrostError, NodePool, cbor::evaluate, common::validate_content_type};
use axum::{Extension, Json, response::IntoResponse};
use hyper::HeaderMap;

use super::model::{TxEvaluationRequest, convert_eval_report};

pub async fn route(
    Extension(node): Extension<NodePool>,
    headers: HeaderMap,
    request_json: Json<TxEvaluationRequest>,
) -> Result<impl IntoResponse, BlockfrostError> {
    // Allow only application/cbor content type
    validate_content_type(&headers, &["application/json"])?;

    let pallas_report = evaluate::evaluate_encoded_tx(node, &request_json.cbor, None).await?;

    Ok(Json(convert_eval_report(pallas_report)))
}
