mod tests {
    use serde_json::json;
    use tower::ServiceExt;

    use crate::common::initialize_app;

    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use reqwest::Method;

    ///
    /// This test is identical to the Blockfrost test:
    /// https://github.com/blockfrost/blockfrost-tests/blob/dc33312126e9d7c49836d7605ab72224a337bc91/src/fixtures/preview/utils/txs-evaluate-utxos.ts#L5
    /// https://github.com/blockfrost/blockfrost-tests/blob/dc33312126e9d7c49836d7605ab72224a337bc91/src/fixtures/preview/utils/txs-evaluate-utxos.ts#L22
    ///
    #[tokio::test]
    async fn success_cbor_only() {
        // init our app
        let app = initialize_app().await;

        let input = json!({
            "cbor": "84A300818258204E9A66B7E310F004893EEF615E11F8AE6C3328CF2BFDB32F6E40063636D42D7C00018182581D70C40F9129C2684046EB02325B96CA2899A6FA6478C1DDE9B5C53206A51A00D59F800200A10581840000D8799F4D48656C6C6F2C20576F726C6421FF820000F5F6",
        });

        // prepare the request
        let request = Request::builder()
            .method(Method::POST)
            .uri("/utils/tx/evaluate/utxos")
            .header("Content-Type", "application/json")
            .body(Body::from(input.to_string()))
            .unwrap();

        // send the request and get the response
        let response = app
            .oneshot(request)
            .await
            .expect("Request to /utils/tx/evaluate failed");

        assert!(response.status().is_success(), "Response should success");

        // Convert the response body to bytes
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");

        // Convert the bytes to a string and print
        let body_str = String::from_utf8_lossy(&body_bytes);

        assert_eq!(
            body_str,
            "{\"methodname\":\"EvaluateTx\",\"reflection\":null,\"result\":{\"spend:0\":{\"cpu\":3776833,\"memory\":15694}},\"servicename\":\"ogmios\",\"type\":\"jsonwsp/response\",\"version\":\"1.0\"}"
        );
    }

    /// This test is identical to the Blockfrost test:
    /// https://github.com/blockfrost/blockfrost-tests/blob/dc33312126e9d7c49836d7605ab72224a337bc91/src/fixtures/preview/utils/txs-evaluate-utxos.ts#L43
    ///
    #[tokio::test]
    #[ignore = "not implemented yet"]
    async fn fail_missing_input() {
        // init our app
        let app = initialize_app().await;

        let input = json!({
            "cbor": "".to_string() +
            "84A30081825820FFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFFF" +
             "FFFFFFFFFFFFFFFF7C00018182581D70C40F9129C2684046EB02325B96CA" +
             "2899A6FA6478C1DDE9B5C53206A51A00D59F800200A10581840000D8799F" +
             "4D48656C6C6F2C20576F726C6421FF820000F5F6",
        "additionalUtxoSet": [

            ]
        });

        // prepare the request
        let request = Request::builder()
            .method(Method::POST)
            .uri("/utils/tx/evaluate/utxos")
            .header("Content-Type", "application/json")
            .body(Body::from(input.to_string()))
            .unwrap();

        // send the request and get the response
        let response = app
            .oneshot(request)
            .await
            .expect("Request to /utils/tx/evaluate failed");
        assert!(response.status().is_success(), "Response should success");

        // Convert the response body to bytes
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");

        // Convert the bytes to a string and print
        let body_str = String::from_utf8_lossy(&body_bytes);

        assert_eq!(
            body_str,
            "{\"error\":\"Bad Request\",\"message\":\"Error evaluating transaction: resolved Input not found\",\"status_code\":400}"
        );
    }
}
