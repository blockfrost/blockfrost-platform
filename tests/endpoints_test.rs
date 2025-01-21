#[path = "common.rs"]
mod common;

mod tests {
    use crate::common::{build_app, initialize_logging};
    use axum::{
        body::{to_bytes, Body},
        http::Request,
    };
    use blockfrost_platform::api::root::RootResponse;
    use pretty_assertions::assert_eq;
    use reqwest::{Method, StatusCode};
    use tower::ServiceExt;

    // Test: `/` route correct response
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

    // Test: `/tx/submit` route error
    #[tokio::test]
    async fn test_submit_route_error() {
        initialize_logging();
        let (app, _handle) = build_app().await.expect("Failed to build the application");

        let tx = "8182068183051a000c275b1a000b35ec";
        let body = Body::from(tx);

        let request = Request::builder()
            .method(Method::POST)
            .uri("/tx/submit")
            .header("Content-Type", "application/cbor")
            .body(body)
            .unwrap();

        let response = app
            .oneshot(request)
            .await
            .expect("Request to /tx/submit failed");

        let cbor = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");

        assert_eq!(cbor, "aaaa");
    }
}
