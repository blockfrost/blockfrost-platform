use base64::{Engine as _, engine::general_purpose};
use serde_json::json;

pub fn generate_reflection_v5() -> serde_json::Value {
    json!({"id": uuid::Uuid::new_v4().to_string()})
}
pub fn generate_reflection_v6() -> serde_json::Value {
    serde_json::Value::String(uuid::Uuid::new_v4().to_string())
}

/// Decodes a CBOR payload from an `application/cbor` endpoint body.
/// Matches Blockfrost API behavior:
/// - Non-ASCII bytes (or empty) → treat as raw binary CBOR
/// - ASCII text → try hex decode, then base64 decode
/// - If neither works → error
pub fn resolve_tx_body(body: &[u8]) -> Result<Vec<u8>, String> {
    let is_text = !body.is_empty() && body.iter().all(|b| b.is_ascii());
    if !is_text {
        return Ok(body.to_vec());
    }

    // 1. Hex decode (even length, all hex chars)
    let even_length = body.len().is_multiple_of(2);
    let all_hex = body.iter().all(|b| b.is_ascii_hexdigit());
    if even_length
        && all_hex
        && let Ok(decoded) = hex::decode(body)
    {
        return Ok(decoded);
    }

    // 2. Base64 decode
    if let Ok(decoded) = general_purpose::STANDARD.decode(body)
        && !decoded.is_empty()
    {
        return Ok(decoded);
    }

    Err("Invalid request: failed to decode payload from base64 or base16.".to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Prod test cases verified via curl against cardano-preview.blockfrost.io

    #[test]
    fn raw_binary_cbor() {
        let cbor = hex::decode("84A300818258204E9A66B7").unwrap();
        assert_eq!(resolve_tx_body(&cbor).unwrap(), cbor);
    }

    #[test]
    fn hex_text_even_length() {
        let hex = b"84A300818258204E9A66B7";
        let expected = hex::decode("84A300818258204E9A66B7").unwrap();
        assert_eq!(resolve_tx_body(hex).unwrap(), expected);
    }

    #[test]
    fn hex_text_odd_length_fails() {
        let odd_hex = b"84a300818258204acdf8c67";
        assert!(resolve_tx_body(odd_hex).is_err());
    }

    #[test]
    fn base64_text() {
        let cbor = hex::decode("84A300818258204E9A66B7").unwrap();
        let b64 = general_purpose::STANDARD.encode(&cbor);
        assert_eq!(resolve_tx_body(b64.as_bytes()).unwrap(), cbor);
    }

    #[test]
    fn garbage_text_fails() {
        assert!(resolve_tx_body(b"hello world").is_err());
    }

    #[test]
    fn empty_body_returns_empty() {
        assert_eq!(resolve_tx_body(b"").unwrap(), Vec::<u8>::new());
    }

    #[test]
    fn raw_0x80_binary() {
        assert_eq!(resolve_tx_body(&[0x80]).unwrap(), vec![0x80]);
    }

    #[test]
    fn hex_text_80_decodes() {
        assert_eq!(resolve_tx_body(b"80").unwrap(), vec![0x80]);
    }
}
