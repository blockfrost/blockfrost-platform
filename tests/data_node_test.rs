mod common;

mod tests {
    use crate::common::{
        build_app_with_data_node, initialize_logging, mock_data_node::MockDataNode,
    };
    use axum::{
        body::{Body, to_bytes},
        http::Request,
    };
    use blockfrost_platform::api::root::RootResponse;
    use pretty_assertions::assert_eq;
    use reqwest::StatusCode;
    use tower::ServiceExt;

    // Test: data_node - health monitoring success
    #[tokio::test]
    #[ntest::timeout(120_000)]
    async fn test_data_node_health_monitoring() {
        initialize_logging();

        let mock = MockDataNode::healthy().await;

        let (app, _, _, _, _) = build_app_with_data_node(mock.url)
            .await
            .expect("Failed to build the application");

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .expect("Request to root route failed");

        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");
        let root_response: RootResponse =
            serde_json::from_slice(&body_bytes).expect("Response body is not valid JSON");

        assert_eq!(root_response.errors, Vec::<String>::new());
        assert!(root_response.healthy);

        assert!(
            root_response.data_node.is_some(),
            "Expected data_node info in response"
        );

        let data_node_info = root_response.data_node.unwrap();

        assert_eq!(data_node_info.version, "0.0.0-test");
        assert_eq!(data_node_info.revision, Some("test-revision".to_string()));
    }

    // Test: data_node - unhealthy status
    #[tokio::test]
    #[ntest::timeout(120_000)]
    async fn test_data_node_unhealthy_status() {
        initialize_logging();

        let mock = MockDataNode::unhealthy().await;

        let (app, _, _, _, _) = build_app_with_data_node(mock.url)
            .await
            .expect("Failed to build the application");

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .expect("Request to root route failed");

        // Should return 503 because data_node is unhealthy
        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");
        let root_response: RootResponse =
            serde_json::from_slice(&body_bytes).expect("Response body is not valid JSON");

        assert!(!root_response.healthy);
        assert!(!root_response.errors.is_empty());
        assert!(
            root_response
                .errors
                .iter()
                .any(|e| e.contains("Data node reports unhealthy status"))
        );
    }

    // Test: data_node - 503 unreachable
    #[tokio::test]
    #[ntest::timeout(120_000)]
    async fn test_data_node_unreachable() {
        initialize_logging();

        let mock = MockDataNode::unreachable();

        let (app, _, _, _, _) = build_app_with_data_node(mock.url)
            .await
            .expect("Failed to build the application");

        let response = app
            .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
            .await
            .expect("Request to root route failed");

        assert_eq!(response.status(), StatusCode::SERVICE_UNAVAILABLE);

        let body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");
        let root_response: RootResponse =
            serde_json::from_slice(&body_bytes).expect("Response body is not valid JSON");

        assert!(!root_response.healthy);
        assert!(!root_response.errors.is_empty());
        assert!(
            root_response
                .errors
                .iter()
                .any(|e| e.contains("Data node unreachable"))
        );
    }
}
