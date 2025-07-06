use crate::NodePool;
use axum::{Extension, Json, http::HeaderMap, response::IntoResponse};
use common::{errors::BlockfrostError, validation::validate_content_type};
use metrics::counter;

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
    let response_body = tokio::spawn(async move {
        // Submit transaction
        let mut node = node.get().await?;
        let response = node.submit_transaction(binary_tx).await;

        if response.is_ok() {
            counter!("tx_submit_success").increment(1)
        } else {
            counter!("tx_submit_failure").increment(1)
        }

        response
    })
    .await
    .expect("submit_transaction panic!")?;

    let mut response_headers = HeaderMap::new();

    response_headers.insert(
        "blockfrost-platform-response",
        response_body.to_string().parse()?,
    );

    Ok((response_headers, Json(response_body)))
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

    if !even_length || xs.iter().any(|&x| !x.is_ascii_hexdigit()) {
        xs.to_vec()
    } else {
        hex::decode(xs).unwrap_or_else(|_| unreachable!())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use proptest::prelude::*;

    proptest! {
        #[test]
        fn proptest_binary_or_hex_heuristic(
            binary in prop::collection::vec(any::<u8>(), 0..=128)
                .prop_filter("exclude values made up only of hex digits", |xs| {
                    let contains_non_hex = xs.iter().any(|&x| !x.is_ascii_hexdigit());
                    contains_non_hex
                })
        ) {
            let hex_string = hex::encode(&binary);
            assert_eq!(
                binary_or_hex_heuristic(hex_string.as_bytes()),
                binary_or_hex_heuristic(&binary)
            )
        }
    }
}
