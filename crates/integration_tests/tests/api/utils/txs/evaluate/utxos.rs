mod tests {
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use integration_tests::platform::initialize_app;
    use reqwest::Method;
    use serde_json::json;
    use tower::ServiceExt;

    /// Conway tx with a single spend redeemer (a "Hello, World!" Plutus V3 script).
    const CONWAY_TX_HEX: &str = "84A300818258204E9A66B7E310F004893EEF615E11F8AE6C3328CF2BFDB32F6E40063636D42D7C00018182581D70C40F9129C2684046EB02325B96CA2899A6FA6478C1DDE9B5C53206A51A00D59F800200A10581840000D8799F4D48656C6C6F2C20576F726C6421FF820000F5F6";

    /// POST `/utils/txs/evaluate/utxos` with the given JSON payload, assert a 2xx,
    /// and return the parsed JSON response.
    async fn post_evaluate_utxos(input: serde_json::Value) -> serde_json::Value {
        let app = initialize_app().await;
        let request = Request::builder()
            .method(Method::POST)
            .uri("/utils/txs/evaluate/utxos")
            .header("Content-Type", "application/json")
            .body(Body::from(input.to_string()))
            .unwrap();
        let response = app
            .oneshot(request)
            .await
            .expect("Request to /utils/txs/evaluate/utxos failed");
        assert!(response.status().is_success(), "Response should success");
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");
        serde_json::from_slice(&body_bytes).unwrap()
    }

    /// This test is identical to the Blockfrost test:
    /// https://github.com/blockfrost/blockfrost-tests/blob/dc33312126e9d7c49836d7605ab72224a337bc91/src/fixtures/preview/utils/txs-evaluate-utxos.ts#L5
    /// https://github.com/blockfrost/blockfrost-tests/blob/dc33312126e9d7c49836d7605ab72224a337bc91/src/fixtures/preview/utils/txs-evaluate-utxos.ts#L22
    #[tokio::test]
    #[ntest::timeout(120_000)]
    async fn success_cbor_only() {
        let body_json = post_evaluate_utxos(json!({ "cbor": CONWAY_TX_HEX })).await;

        assert_eq!(body_json["type"], "jsonwsp/response");
        assert_eq!(body_json["servicename"], "ogmios");
        assert_eq!(body_json["methodname"], "EvaluateTx");
        assert!(body_json["reflection"]["id"].is_string());
        assert_eq!(
            body_json["result"],
            json!({"EvaluationResult": {"spend:0": {"memory": 15694, "steps": 3776164}}})
        );
    }

    /// Verifies that the mirror field sent by the client is NOT echoed back.
    #[tokio::test]
    #[ntest::timeout(120_000)]
    async fn success_mirror_not_echoed() {
        let body_json = post_evaluate_utxos(json!({
            "cbor": CONWAY_TX_HEX,
            "mirror": {"id": "test-request-123"},
        }))
        .await;

        assert_eq!(body_json["type"], "jsonwsp/response");
        assert!(body_json["reflection"]["id"].is_string());
        assert_ne!(body_json["reflection"]["id"], "test-request-123");
    }

    /// This test is identical to the Blockfrost test:
    /// https://github.com/blockfrost/blockfrost-tests/blob/dc33312126e9d7c49836d7605ab72224a337bc91/src/fixtures/preview/utils/txs-evaluate-utxos.ts#L43
    #[tokio::test]
    #[ntest::timeout(120_000)]
    #[ignore = "not implemented yet"]
    async fn fail_missing_input() {
        let body_json = post_evaluate_utxos(json!({
            "cbor": "84A30081825820FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF7C00018182581D70C40F9129C2684046EB02325B96CA2899A6FA6478C1DDE9B5C53206A51A00D59F800200A10581840000D8799F4D48656C6C6F2C20576F726C6421FF820000F5F6",
            "additionalUtxoSet": [],
        }))
        .await;

        assert_eq!(
            body_json,
            json!({
                "error": "Bad Request",
                "message": "Error evaluating transaction: resolved Input not found",
                "status_code": 400,
            })
        );
    }
}
