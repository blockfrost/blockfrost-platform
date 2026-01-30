mod common;

mod tests {
    use crate::common::{build_app, initialize_logging};
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use blockfrost_platform::api::root::RootResponse;
    use pretty_assertions::assert_eq;
    use reqwest::StatusCode;
    use tower::ServiceExt;

    // Test: `/` route correct response
    #[tokio::test]
    #[ntest::timeout(120_000)]
    async fn test_route_root() {
        initialize_logging();

        let (app, _, _, _, _) = build_app().await.expect("Failed to build the application");

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .expect("Request to root route failed");

        assert_eq!(response.status(), StatusCode::OK);

        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");
        let root_response: RootResponse =
            serde_json::from_slice(&body_bytes).expect("Response body is not valid JSON");

        assert!(root_response.errors.is_empty());
        assert_eq!(root_response.name, "blockfrost-platform");
        assert!(root_response.healthy);
        assert_eq!(root_response.node_info.unwrap().sync_progress, 100.0);
        // data_node is not configured in build_app()
        assert!(root_response.data_node.is_none());
    }
}
