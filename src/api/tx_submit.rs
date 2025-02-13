use crate::{common::validate_content_type, BlockfrostError, NodePool};
use axum::{http::HeaderMap, response::IntoResponse, Extension, Json};

pub async fn route(
    Extension(node): Extension<NodePool>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<impl IntoResponse, BlockfrostError> {
    // Allow only application/cbor content type
    validate_content_type(&headers, &["application/cbor"])?;

    // Submit transaction
    let mut node = node.get().await?;
    let response = node.submit_transaction(body.as_ref()).await?;

    Ok(Json(response))
}
