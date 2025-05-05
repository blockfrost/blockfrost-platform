mod tests {
    use tower::ServiceExt;

    use crate::common::{get_blockfrost_client, initialize_app};
    use crate::tx_builder::build_tx;

    use axum::{body::Body, http::Request};
    use reqwest::Method;

    #[tokio::test]
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
}
