mod tests {
    use axum::body::to_bytes;
    use tower::ServiceExt;

    use crate::common::{get_blockfrost_client, initialize_app};
    use crate::tx_builder::build_tx;

    use axum::{body::Body, http::Request};
    use reqwest::Method;

    #[tokio::test]
    #[ignore = "not implemented yet"]
    async fn success() {
        // prepare the transaction
        let blockfrost_client = get_blockfrost_client();
        let tx = build_tx(&blockfrost_client).await.unwrap();

        // init our app
        let app = initialize_app().await;

        // prepare the request
        let request = Request::builder()
            .method(Method::POST)
            .uri("/utils/tx/evaluate")
            .header("Content-Type", "application/cbor")
            .body(Body::from(tx.to_hex()))
            .unwrap();

        // send the request and get the response
        let response = app
            .oneshot(request)
            .await
            .expect("Request to /utils/tx/evaluate failed");

        assert_eq!(200, response.status());
    }

    ///
    /// This test is identical to the Blockfrost test:
    /// https://github.com/blockfrost/blockfrost-tests/blob/7a847bc41b8153844a2643d817559367cc4ffd4d/src/fixtures/preview/utils/txs-evaluate.ts#L5
    ///
    #[tokio::test]
    #[ignore = "not implemented yet"]
    async fn blockfrost_test() {
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

        // Convert the bytes to a string and print
        let body_str = String::from_utf8_lossy(&body_bytes);

        assert_eq!(body_str, "this should contain eval failure");
    }
}
