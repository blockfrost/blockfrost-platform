mod common;

#[cfg(test)]
mod root_tests {
    use crate::common::{build_app, initialize_logging};
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use blockfrost_platform::api::root::RootResponse;
    use pretty_assertions::assert_eq;
    use reqwest::StatusCode;
    use tower::ServiceExt;

    #[tokio::test]
    async fn test_root_route() {
        initialize_logging();

        let (app, _handle) = build_app().await.expect("Failed to build the application");

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
        assert_eq!(root_response.sync_progress.percentage, 100.0);
    }
}
