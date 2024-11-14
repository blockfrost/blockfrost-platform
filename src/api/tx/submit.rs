use crate::{
    common::validate_content_type,
    errors::BlockfrostError,
    node::{pool::NodeConnPool, transactions::submit_transaction},
};
use axum::{http::HeaderMap, response::IntoResponse, Extension, Json};

pub async fn route(
    Extension(node): Extension<NodeConnPool>,
    headers: HeaderMap,
    body: String,
) -> Result<impl IntoResponse, BlockfrostError> {
    // Allow only application/cbor content type
    validate_content_type(&headers, &["application/cbor"])?;

    // Submit transaction
    let mut node = node.get().await?;
    let response = submit_transaction(&mut node, body).await?;

    Ok(Json(response))
}
