mod asserts;
mod common;
mod tx_builder;

mod tests {
    use std::net::{IpAddr, SocketAddr};

    use crate::asserts;
    use crate::common::{
        build_app, build_app_non_solitary, get_blockfrost_client, initialize_logging,
    };
    use crate::tx_builder::build_tx;
    use axum::ServiceExt as AxumServiceExt;
    use axum::{
        body::{Body, to_bytes},
        extract::Request as AxumExtractRequest,
        http::Request,
    };
    use blockfrost_platform::api::root::RootResponse;
    use pretty_assertions::assert_eq;
    use reqwest::{Method, StatusCode};
    use tokio::sync::oneshot;
    use tower::ServiceExt;
    use tracing::info;

    // Test: `/` route correct response
    #[tokio::test]
    async fn test_route_root() {
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
        assert_eq!(root_response.node_info.unwrap().sync_progress, 100.0);
    }

    // Test: `/metrics` route sanity check
    #[tokio::test]
    async fn test_route_metrics() {
        initialize_logging();

        let (app, _, _, _) = build_app().await.expect("Failed to build the application");

        let response = app
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
    }

    // Test: `/tx/submit` error has same response as blockfrost API
    #[tokio::test]
    async fn test_route_submit() {
        initialize_logging();
        let (app, _, _, _) = build_app().await.expect("Failed to build the application");

        let tx = "84a300d90102818258205176274bef11d575edd6aa72392aaf993a07f736e70239c1fb22d4b1426b22bc01018282583900ddf1eb9ce2a1561e8f156991486b97873fb6969190cbc99ddcb3816621dcb03574152623414ed354d2d8f50e310f3f2e7d167cb20e5754271a003d09008258390099a5cb0fa8f19aba38cacf8a243d632149129f882df3a8e67f6bd512bcb0cde66a545e9fbc7ca4492f39bca1f4f265cc1503b4f7d6ff205c1b000000024f127a7c021a0002a2ada100d90102818258208b83e59abc9d7a66a77be5e0825525546a595174f8b929f164fcf5052d7aab7b5840709c64556c946abf267edd90b8027343d065193ef816529d8fa7aa2243f1fd2ec27036a677974199e2264cb582d01925134b9a20997d5a734da298df957eb002f5f6";

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
            .body(hex::decode(tx).unwrap())
            .send()
            .await
            .expect("Blockfrost request failed");

        let bf_body_bytes = bf_response
            .bytes()
            .await
            .expect("Failed to read Blockfrost response");

        asserts::assert_submit_error_responses(&bf_body_bytes, &local_body_bytes);
    }

    // Test: build `/tx/submit` success - tx is accepted by the node
    #[tokio::test]
    async fn test_submit_route_success() {
        initialize_logging();
        let (app, _, _, _) = build_app().await.expect("Failed to build the application");
        let blockfrost_client = get_blockfrost_client();
        let tx = build_tx(&blockfrost_client).await.unwrap();

        let request = Request::builder()
            .method(Method::POST)
            .uri("/tx/submit")
            .header("Content-Type", "application/cbor")
            .body(Body::from(tx.to_hex()))
            .unwrap();

        let response = app
            .oneshot(request)
            .await
            .expect("Request to /tx/submit failed");

        assert!(
            response
                .headers()
                .contains_key("blockfrost-platform-response"),
            "Response is missing the `blockfrost-platform-response` header"
        );

        let local_body_bytes = to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("Failed to read response body");

        assert_eq!(66, local_body_bytes.len());
    }

    // Test: `icebreakers register` success registration
    #[tokio::test]
    async fn test_icebreakers_registrations() -> Result<(), Box<dyn std::error::Error>> {
        initialize_logging();

        let (app, _, icebreakers_api, api_prefix) = build_app_non_solitary()
            .await
            .expect("Failed to build the application");

        let ip_addr: IpAddr = "0.0.0.0".parse().unwrap();
        let address = SocketAddr::new(ip_addr, 3000);
        let listener = tokio::net::TcpListener::bind(address).await?;
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let (ready_tx, ready_rx) = oneshot::channel();

        let spawn_task = tokio::spawn({
            let app = app;
            async move {
                let server_future = axum::serve(
                    listener,
                    AxumServiceExt::<AxumExtractRequest>::into_make_service(app),
                )
                .with_graceful_shutdown(async {
                    shutdown_rx.await.ok();
                });

                let _ = ready_tx.send(());
                server_future.await
            }
        });

        if ready_rx.await.is_ok() {
            info!("Server is listening on http://{}{}", address, api_prefix);

            if let Some(api) = &icebreakers_api {
                api.register().await?;
            }
        }

        let _ = shutdown_tx.send(());

        spawn_task.await.unwrap().unwrap();

        Ok(())
    }
}
