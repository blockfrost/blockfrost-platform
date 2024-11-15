use crate::errors::BlockfrostError;
use axum::http::{header::CONTENT_TYPE, HeaderMap};

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
}
