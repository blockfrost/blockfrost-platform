mod common;

mod tests {
    use crate::common::{build_app, initialize_logging};
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use reqwest::StatusCode;
    use tower::ServiceExt;

    // Test: `/metrics` route sanity check and trailing slash
    #[tokio::test]
    #[ntest::timeout(120_000)]
    async fn test_route_metrics() {
        initialize_logging();

        let (app, _, _, _, _) = build_app().await.expect("Failed to build the application");

        // Test without trailing slash
        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .uri("/metrics")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("Request to /metrics route failed");

        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");

        let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
        assert!(body_str.contains("cardano_node_connections"));

        // Test with trailing slash
        let response_trailing = app
            .oneshot(
                Request::builder()
                    .uri("/metrics/")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .expect("Request to /metrics/ route failed");

        assert_eq!(response_trailing.status(), StatusCode::OK);

        let body_bytes_trailing = to_bytes(response_trailing.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body for /metrics/");

        let body_str_trailing = String::from_utf8(body_bytes_trailing.to_vec()).unwrap();

        assert!(body_str_trailing.contains("cardano_node_connections"));
    }
}
