mod tests {
    use axum::body::to_bytes;
    use axum::{body::Body, http::Request};
    use integration_tests::platform::initialize_app;
    use reqwest::Method;
    use rstest::rstest;
    use serde_json::json;
    use tower::ServiceExt;

    /// Conway tx with a single spend redeemer (a "Hello, World!" Plutus V3 script).
    const CONWAY_TX_HEX: &str = "84A300818258204E9A66B7E310F004893EEF615E11F8AE6C3328CF2BFDB32F6E40063636D42D7C00018182581D70C40F9129C2684046EB02325B96CA2899A6FA6478C1DDE9B5C53206A51A00D59F800200A10581840000D8799F4D48656C6C6F2C20576F726C6421FF820000F5F6";

    /// Pre-Alonzo (Mary era) tx, encoded as a 3-element CBOR array.
    const MARY_TX_HEX: &str = "83a300818258200ac82ea5bc0967a17d4a60e2474b01df72440673429ff89b2802d3bd2a38ec3e01018282583900e2fbc47df26fcd065c074c451e792599ea8fc159f76163ca4c2b520b58adbef896164ee7456ccb4eaa965a87a602b0e3b2825d7b4ee789b01a000f4240825839003c77cd7f3c07b3b0ba72044848592d2e5687569ad25b93a926392f5e83892080b40900e146e1c68f12ef6811773bd8740196cd211f3211de1af9b0595d021a0002c5bda10081825820da818bbf3a082945884681d062147ca7dc3111d87fab415268749124a3ed1d31584059ca300a7d38abf454482a57281acdbbaab740b868978131f36117a224e6ba2be5248da0205296d7a8211506d6430a2873c201831e326e5db68ac9e1403e520ef6";

    /// POST `/utils/txs/evaluate` with the given query string and CBOR-hex body,
    /// assert a 2xx, and return the parsed JSON response.
    async fn post_evaluate(query: &str, tx_hex: &str) -> serde_json::Value {
        let app = initialize_app().await;
        let uri = if query.is_empty() {
            "/utils/txs/evaluate".to_string()
        } else {
            format!("/utils/txs/evaluate?{query}")
        };
        let request = Request::builder()
            .method(Method::POST)
            .uri(uri)
            .header("Content-Type", "application/cbor")
            .body(Body::from(tx_hex.to_string()))
            .unwrap();
        let response = app
            .oneshot(request)
            .await
            .expect("Request to /utils/txs/evaluate failed");
        assert!(
            response.status().is_success(),
            "Response was not successful"
        );
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");
        serde_json::from_slice(&body_bytes).unwrap()
    }

    #[rstest]
    #[case::no_version("")]
    #[case::v5("version=5")]
    #[case::unknown_version_falls_back("version=53")]
    #[tokio::test]
    #[ntest::timeout(120_000)]
    async fn test_success_v5(#[case] query: &str) {
        let body_json = post_evaluate(query, CONWAY_TX_HEX).await;

        assert_eq!(body_json["type"], "jsonwsp/response");
        assert_eq!(body_json["version"], "1.0");
        assert_eq!(body_json["servicename"], "ogmios");
        assert_eq!(body_json["methodname"], "EvaluateTx");
        assert!(body_json["reflection"]["id"].is_string());
        assert_eq!(
            body_json["result"],
            json!({"EvaluationResult": {"spend:0": {"memory": 15694, "steps": 3776164}}})
        );
    }

    #[tokio::test]
    #[ntest::timeout(120_000)]
    #[ignore = "pallas-validate does not generate same results with the external evaluator (ledger)"]
    async fn test_success_v5_native() {
        let body_json = post_evaluate("version=5&evaluator=native", CONWAY_TX_HEX).await;

        assert_eq!(body_json["type"], "jsonwsp/response");
        assert_eq!(body_json["servicename"], "ogmios");
        assert_eq!(body_json["methodname"], "EvaluateTx");
        assert!(body_json["reflection"]["id"].is_string());
        assert_eq!(
            body_json["result"],
            json!({"EvaluationResult": {"spend:0": {"memory": 15694, "steps": 3776164}}})
        );
    }

    #[tokio::test]
    #[ntest::timeout(120_000)]
    async fn test_success_v6() {
        let body_json = post_evaluate("version=6", CONWAY_TX_HEX).await;

        assert_eq!(body_json["jsonrpc"], "2.0");
        assert_eq!(body_json["method"], "evaluateTransaction");
        assert!(body_json["id"].is_string());
        assert_eq!(
            body_json["result"],
            json!([{"budget": {"cpu": 3776164, "memory": 15694}, "validator": {"purpose": "spend", "index": 0}}])
        );
    }

    /// Pre-Alonzo (Mary era) transaction returns IncompatibleEra in v5 format
    #[tokio::test]
    #[ntest::timeout(120_000)]
    async fn test_fail_incompatible_era_v5() {
        let body_json = post_evaluate("version=5", MARY_TX_HEX).await;

        assert_eq!(
            body_json.get("type").unwrap().as_str().unwrap(),
            "jsonwsp/response"
        );
        assert_eq!(
            body_json["result"]["EvaluationFailure"]["IncompatibleEra"]
                .as_str()
                .unwrap(),
            "Mary"
        );
    }

    /// Pre-Alonzo (Mary era) transaction returns IncompatibleEra in v6 format (code 3000)
    #[tokio::test]
    #[ntest::timeout(120_000)]
    async fn test_fail_incompatible_era_v6() {
        let body_json = post_evaluate("version=6", MARY_TX_HEX).await;

        let error = body_json.get("error").unwrap();
        assert_eq!(error.get("code").unwrap().as_i64().unwrap(), 3000);
        assert_eq!(error["data"]["incompatibleEra"].as_str().unwrap(), "mary");
    }
}
