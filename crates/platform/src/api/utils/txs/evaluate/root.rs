use crate::validation::validate_content_type;
use crate::{BlockfrostError, api::utils::txs::evaluate::model::EvaluateQuery};
use axum::{Extension, Json, extract::Query, response::IntoResponse};
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
    validate_content_type(&headers, &["application/cbor"])?;

    match query.version {
        6 => Ok(Json(
            evaluator
                .evaluate_tx_payload_v6(node, body.as_ref(), None)
                .await?,
        )),
        // Everything else is treated as v5
        _ => Ok(Json(
            evaluator
                .evaluate_tx_payload_v5(node, body.as_ref(), None)
                .await?,
        )),
    }
}
