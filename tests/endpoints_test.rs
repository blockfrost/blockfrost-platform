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

        let (app, _, _, _) = build_app().await.expect("Failed to build the application");

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
        assert_eq!(root_response.node_info.sync_progress, 100.0);
    }

    // Test: `/tx/submit` error has same response as blockfrost API
    #[tokio::test]
    async fn test_submit_route_error() {
        initialize_logging();
        let (app, _, _, _) = build_app().await.expect("Failed to build the application");

        let tx =    "84a300d90102818258205176274bef11d575edd6aa72392aaf993a07f736e70239c1fb22d4b1426b22bc01018282583900ddf1eb9ce2a1561e8f156991486b97873fb6969190cbc99ddcb3816621dcb03574152623414ed354d2d8f50e310f3f2e7d167cb20e5754271a003d09008258390099a5cb0fa8f19aba38cacf8a243d632149129f882df3a8e67f6bd512bcb0cde66a545e9fbc7ca4492f39bca1f4f265cc1503b4f7d6ff205c1b000000024f127a7c021a0002a2ada100d90102818258208b83e59abc9d7a66a77be5e0825525546a595174f8b929f164fcf5052d7aab7b5840709c64556c946abf267edd90b8027343d065193ef816529d8fa7aa2243f1fd2ec27036a677974199e2264cb582d01925134b9a20997d5a734da298df957eb002f5f6";

        // Local (Platform)
        let local_request = Request::builder()
            .method(Method::POST)
            .uri("/tx/submit")
            .header("Content-Type", "application/cbor")
            .body(Body::from(tx))
            .unwrap();

        let local_response = app
            .oneshot(local_request)
            .await
            .expect("Request to /tx/submit failed");

        let local_body_bytes = to_bytes(local_response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");

        // Blockfrost API
        let bf_response = reqwest::Client::new()
            .post("https://cardano-preview.blockfrost.io/api/v0/tx/submit")
            .header("Content-Type", "application/cbor")
            .header("project_id", "previewWrlEvs2PlZUw8hEN5usP5wG4DK4L46A3")
            .body(tx)
            .send()
            .await
            .expect("Blockfrost request failed");

        let bf_body_bytes = bf_response
            .bytes()
            .await
            .expect("Failed to read Blockfrost response");

        // TODO: https://github.com/blockfrost/blockfrost-platform/issues/19
        // assert_eq!(local_body_bytes, bf_body_bytes);
        // This should be uncommented when the issue is resolved
        println!("bf response {:?}", bf_body_bytes);
        println!("local response {:?}", local_body_bytes);

        let local_body_str = String::from_utf8_lossy(&local_body_bytes);

        // Uncomment this to see the difference between the blockfrost response and platform response
        // assert_eq!(bf_body_bytes, local_body_bytes,);

        assert!(
            local_body_str.contains("BadInputsUTxO"),
            "Expected 'BadInputsUTxO' in the local response, but it was not found."
        );
    }
}
