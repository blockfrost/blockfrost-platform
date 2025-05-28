use crate::BlockfrostError;
use axum::http::{HeaderMap, header::CONTENT_TYPE};
use pallas_network::miniprotocols::localstate::queries_v16::BigInt;

/// Helper to validate content type or return custom BlockfrostError 400
///   Arguments:
/// * headers: &HeaderMap - headers from request
/// * allowed_headers: &[&str] - allowed content types
///   Returns:
/// * Result<bool, BlockfrostError> - true if content type is valid
/// * BlockfrostError - custom 400 error if content type is invalid
pub fn validate_content_type(
    headers: &HeaderMap,
    allowed_content_types: &[&str],
) -> Result<bool, BlockfrostError> {
    if let Some(content_type) = headers.get(CONTENT_TYPE) {
        let is_valid_type = allowed_content_types
            .iter()
            .any(|&allowed_type| allowed_type == content_type);

        if !is_valid_type {
            let error_message = if allowed_content_types.len() == 1 {
                format!("Content-Type must be: {:?}", allowed_content_types[0])
            } else {
                format!("Content-Type must be one of: {:?}", allowed_content_types)
            };

            return Err(BlockfrostError::custom_400(error_message));
        }
    }

    Ok(true)
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
        hex::decode(xs).unwrap_or_else(|_| unreachable!())
    }
}

pub fn convert_bigint(bigint: BigInt) -> Result<i128, BlockfrostError> {
    match bigint {
        BigInt::Int(big) => Ok(i128::from(big)),
        _ => Err(BlockfrostError::internal_server_error(
            "Invalid/unsupported BigInt format".to_string(),
        )),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    use pretty_assertions::assert_eq;
    use rstest::rstest;

    #[rstest]
    #[case(&["application/json"], "application/json", true, None)]
    #[case(&["application/json", "application/xml"], "application/json", true, None)]
    #[case(&["application/json"], "application/xml", false, Some("BlockfrostError: Content-Type must be: \"application/json\""))]
    #[case(&["application/json", "application/xml"], "text/html", false, Some("BlockfrostError: Content-Type must be one of: [\"application/json\", \"application/xml\"]"))]
    #[case(&["application/json"], "", true, None)]
    #[case(&[], "application/json", false, Some("BlockfrostError: Content-Type must be one of: []"))]
    fn test_validate_content_type(
        #[case] allowed_headers: &[&str],
        #[case] content_type: &str,
        #[case] expected_ok: bool,
        #[case] expected_err: Option<&str>,
    ) {
        use axum::http::HeaderValue;
        let mut headers = HeaderMap::new();

        if !content_type.is_empty() {
            headers.insert(CONTENT_TYPE, HeaderValue::from_str(content_type).unwrap());
        }

        let result = validate_content_type(&headers, allowed_headers);

        if expected_ok {
            assert!(result.is_ok());
        } else {
            assert!(result.is_err());
            if let Some(expected_err_msg) = expected_err {
                if let Err(e) = result {
                    assert_eq!(e.to_string(), expected_err_msg);
                }
            }
        }
    }

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
