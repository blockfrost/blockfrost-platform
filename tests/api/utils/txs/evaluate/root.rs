mod tests {
    use axum::body::to_bytes;
    use tower::ServiceExt;

    use crate::common::initialize_app;

    use axum::{body::Body, http::Request};
    use reqwest::Method;

    ///
    /// Currently not working since we are not handling desirializing the tx for old eras on Haskell side.
    /// This test is identical to the Blockfrost test:
    /// https://github.com/blockfrost/blockfrost-tests/blob/7a847bc41b8153844a2643d817559367cc4ffd4d/src/fixtures/preview/utils/txs-evaluate.ts#L5
    ///
    #[tokio::test]
    #[ignore = "not implemented yet"]
    async fn test_fail_incompatible_era() {
        let tx_hex = "83a300818258200ac82ea5bc0967a17d4a60e2474b01df72440673429ff89b2802d3bd2a38ec3e01018282583900e2fbc47df26fcd065c074c451e792599ea8fc159f76163ca4c2b520b58adbef896164ee7456ccb4eaa965a87a602b0e3b2825d7b4ee789b01a000f4240825839003c77cd7f3c07b3b0ba72044848592d2e5687569ad25b93a926392f5e83892080b40900e146e1c68f12ef6811773bd8740196cd211f3211de1af9b0595d021a0002c5bda10081825820da818bbf3a082945884681d062147ca7dc3111d87fab415268749124a3ed1d31584059ca300a7d38abf454482a57281acdbbaab740b868978131f36117a224e6ba2be5248da0205296d7a8211506d6430a2873c201831e326e5db68ac9e1403e520ef6";
        // init our app
        let app = initialize_app().await;

        // prepare the request
        let request = Request::builder()
            .method(Method::POST)
            .uri("/utils/tx/evaluate")
            .header("Content-Type", "application/cbor")
            .body(Body::from(tx_hex))
            .unwrap();

        // send the request and get the response
        let response = app
            .oneshot(request)
            .await
            .expect("Request to /utils/tx/evaluate failed");
        assert!(
            response.status().is_success(),
            "Response was not successful"
        );

        // Convert the response body to bytes
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");

        let body_str = String::from_utf8_lossy(&body_bytes);

        assert_eq!(
            body_str,
            "[{\"EvaluationFailure\":{\"IncompatibleEra\":\"Mary\"}}]"
        );
    }

    /// Currently not working, we need to mimic Haskell error on the Rust part.
    /// This test is identical to the Blockfrost test:
    /// https://github.com/blockfrost/blockfrost-tests/blob/7a847bc41b8153844a2643d817559367cc4ffd4d/src/fixtures/preview/utils/txs-evaluate.ts#L20
    ///
    #[tokio::test]
    #[ignore = "not implemented yet"]
    async fn test_fail_ill_formed_tx() {
        let tx_hex = "83a300818258200ac82ea5bc0967a17d4a60e2474b01df72440673429ff89b2802d3bd2a38ec3e01018282583900e2fbc47df26fcd065c074c451e792599ea8fc159f76163ca4c2b520b58adbef896164ee7456ccb4eaa965a87a602b0e3b2825d7b4ee789b01a000f4240825839003c77cd7f3c07b3b0ba72044848592d2e5687569ad25b93a926392f5e83892080b40900e146e1c68f12ef6811773bd8740196cd211f3211de1af9b0595d021a0002c5bda10081825820da818bbf3a082945884681d062147ca7dc3111d87fab415268749124a3ed1d31584059ca300a7d38abf454482a57281acdbbaab740b868978131f36117a224e6ba2be5248da0205296d7a8211506d6430a2873c201831e326e5db68ac9e1403e520ef6";
        // init our app
        let app = initialize_app().await;

        // prepare the request
        let request = Request::builder()
            .method(Method::POST)
            .uri("/utils/tx/evaluate")
            .header("Content-Type", "application/cbor")
            .body(Body::from(tx_hex))
            .unwrap();

        // send the request and get the response
        let response = app
            .oneshot(request)
            .await
            .expect("Request to /utils/tx/evaluate failed");
        assert!(
            response.status().is_success(),
            "Response was not successful"
        );

        // Convert the response body to bytes
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");

        let body_str = String::from_utf8_lossy(&body_bytes);

        assert_eq!(
            body_str,
            "[Invalid request: Deserialisation failure while decoding serialised transaction. CBOR failed with error: DeserialiseFailure 0 \"expected tag\"."
        );
    }

    #[tokio::test]
    async fn test_success() {
        let tx_hex = "84A300818258204E9A66B7E310F004893EEF615E11F8AE6C3328CF2BFDB32F6E40063636D42D7C00018182581D70C40F9129C2684046EB02325B96CA2899A6FA6478C1DDE9B5C53206A51A00D59F800200A10581840000D8799F4D48656C6C6F2C20576F726C6421FF820000F5F6";
        // init our app
        let app = initialize_app().await;

        // prepare the request
        let request = Request::builder()
            .method(Method::POST)
            .uri("/utils/tx/evaluate")
            .header("Content-Type", "application/cbor")
            .body(Body::from(tx_hex))
            .unwrap();

        // send the request and get the response
        let response = app
            .oneshot(request)
            .await
            .expect("Request to /utils/tx/evaluate failed");
        assert!(
            response.status().is_success(),
            "Response was not successful"
        );

        // Convert the response body to bytes
        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");

        // Convert the bytes to a string and print
        let body_str = String::from_utf8_lossy(&body_bytes);

        assert_eq!(
            body_str,
            "[{\"budget\":{\"cpu\":3776833,\"memory\":15694},\"validator\":\"spend:0\"}]"
        );
    }
}
