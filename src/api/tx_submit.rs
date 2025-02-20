use crate::{common::validate_content_type, BlockfrostError, NodePool};
use axum::{http::HeaderMap, response::IntoResponse, Extension, Json};
use metrics::gauge;

pub async fn route(
    Extension(node): Extension<NodePool>,
    headers: HeaderMap,
    body: axum::body::Bytes,
) -> Result<impl IntoResponse, BlockfrostError> {
    // Allow only application/cbor content type
    validate_content_type(&headers, &["application/cbor"])?;

    // Allow both hex-encoded and raw binary bodies
    let binary_tx = binary_or_hex_heuristic(body.as_ref());

    // XXX: Axum must not abort Ouroboros protocols in the middle, hence a separate Tokio task:
    let response = tokio::spawn(async move {
        // Submit transaction
        let mut node = node.get().await?;
        let response = node.submit_transaction(binary_tx).await;

        if response.is_ok() {
            gauge!("tx_submit_success").increment(1)
        } else {
            gauge!("tx_submit_failure").increment(1)
        }

        response
    })
    .await
    .expect("submit_transaction panic!")?;

    Ok(Json(response))
}

/// This function allows us to take both hex-encoded and raw bytes. It has
/// to be a heuristic: if there are input bytes that are not `[0-9a-f]`,
/// then it must be a binary string. Otherwise, we assume it’s hex encoded.
///
/// **Note**: there is a small probability that the user gave us a binary
/// string that only _looked_ like a hex-encoded one, but it’s rare enough
/// to ignore it.
pub fn binary_or_hex_heuristic(xs: &[u8]) -> Vec<u8> {
    let even_length = xs.len() % 2 == 0;
    let contains_non_hex = xs.iter().any(|&x| !x.is_ascii_hexdigit());
    if !even_length || contains_non_hex {
        xs.to_vec()
    } else {
        hex::decode(xs).expect("can't happen")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn proptest_binary_or_hex_heuristic(binary in prop::collection::vec(any::<u8>(), 0..=128)) {
            let hex_string = hex::encode(&binary);
            assert_eq!(
                binary_or_hex_heuristic(hex_string.as_bytes()),
                binary_or_hex_heuristic(&binary)
            )
        }
    }
}
