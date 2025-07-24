use crate::{
    BlockfrostError
};
use axum::{Extension, Json, extract, response::IntoResponse};
use common::validation::validate_content_type;
use hyper::HeaderMap;
use node::pool::NodePool;
use tx_evaluator::{external::ExternalEvaluator, model::TxEvaluationRequest};

pub async fn route(
    Extension(node): Extension<NodePool>,
    Extension(fallback_evaluator): Extension<ExternalEvaluator>,
    headers: HeaderMap,
    extract::Json(tx_request): extract::Json<TxEvaluationRequest>,
) -> Result<impl IntoResponse, BlockfrostError> {
    // Allow only application/json content type
    validate_content_type(&headers, &["application/json"])?;

    let tx_cbor = hex::decode(tx_request.cbor).unwrap();

    let result = fallback_evaluator
        .evaluate_binary_tx(node, tx_cbor.as_slice(), tx_request.additional_utxo_set)
        .await?;

    Ok(Json(result))
}
