mod asserts;
mod common;
mod tx_builder;

mod tests {
    use blockfrost_platform::BlockfrostError;
    use std::net::{IpAddr, SocketAddr};
    use std::sync::Arc;
    use tokio::sync::Mutex;

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
    use blockfrost_platform::icebreakers::manager::IcebreakersManager;
    use pretty_assertions::assert_eq;
    use reqwest::{Method, StatusCode};
    use tokio::sync::oneshot;
    use tower::ServiceExt;
    use tracing::info;

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
    }

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

    // Test: `/tx/submit` error has same response as blockfrost API
    #[tokio::test]
    #[ntest::timeout(120_000)]
    async fn test_route_submit_cbor_error() {
        initialize_logging();
        let (app, _, _, _, _) = build_app().await.expect("Failed to build the application");

        let tx = "AAAAAA";

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

        let local_body_str = String::from_utf8(local_body_bytes.to_vec())
            .expect("Failed to convert bytes to string");

        let expected = r#"{"error":"Bad Request","message":"{\"tag\":\"TxSubmitFail\",\"contents\":{\"tag\":\"TxCmdTxReadError\",\"contents\":[\"DecoderErrorDeserialiseFailure \\\"Shelley Tx\\\" (DeserialiseFailure 0 (\\\"unexpected type map at position 0: expected array\\\"))\"]}}","status_code":400}"#;
        assert_eq!(expected, &local_body_str);
    }

    // Test: `/tx/submit` error has same response as blockfrost API
    #[tokio::test]
    #[ntest::timeout(120_000)]
    async fn test_route_submit_error() {
        initialize_logging();
        let (app, _, _, _, _) = build_app().await.expect("Failed to build the application");

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

    // validation of the fix: https://github.com/blockfrost/blockfrost-platform/issues/238#issuecomment-2747354365
    #[tokio::test]
    #[ntest::timeout(120_000)]
    async fn test_route_submit_agent_dequeu() {
        initialize_logging();
        let (app, _, _, _, _) = build_app().await.expect("Failed to build the application");

        let tx = "84a800848258204c16d304e6d531c59afd87a9199b7bb4175bc131b3d6746917901046b662963c00825820893c3f630c0b2db16d041c388aa0d58746ccbbc44133b2d7a3127a72c79722f1018258200998adb591c872a241776e39fe855e04b2d7c361008e94c582f59b6b6ccc452c028258208380ce7240ba59187f6450911f74a70cf3d2749228badb2e7cd10fb6499355f503018482581d61e15900a9a62a8fb01f936a25bf54af209c7ed1248c4e5abd05ec4e76821a0023ba63a1581ca0028f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c235a145484f534b5900a300581d71cba5c6770fe7b30ebc1fa32f01938c150513211360ded23ac76e36b301821a006336d5a3581c239075b83c03c2333eacd0b0beac6b8314f11ce3dc0c047012b0cad4a144706f6f6c01581c3547b4325e495d529619335603ababde10025dceafa9ed34b1fb6611a158208b284793d3bd4967244a2ddd68410d56d06d36ac8d201429b937096a2e8234bc1b7ffffffffffade6b581ca0028f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c235a145484f534b59195e99028201d818583ad8799fd8799f4040ffd8799f581ca0028f350aaabe0545fdcb56b039bfb08e4bb4d8c4d7c3c7d481c23545484f534b59ff1a006336d5195e99ff825839016d06090559d8ed2988aa5b2fff265d668cf552f4f62278c0128f816c0a48432e080280d0d9b15edb65563995f97ce236035afea568e660d1821a00118f32a1581c2f8b2d1f384485896f38406173fa11df2a4ce53b4b0886138b76597aa1476261746368657201825839016d06090559d8ed2988aa5b2fff265d668cf552f4f62278c0128f816c0a48432e080280d0d9b15edb65563995f97ce236035afea568e660d11a06d9f713021a000ab9e00b582027f17979d848d6472896266dd8bf39f7251ca23798713464bc407bf637286c230d81825820cf5de9189b958f8ad64c1f1837c2fa4711d073494598467a1c1a59589393eae20310825839016d06090559d8ed2988aa5b2fff265d668cf552f4f62278c0128f816c0a48432e080280d0d9b15edb65563995f97ce236035afea568e660d11a08666c75111a001016d01282825820bf93dc59c10c19c35210c2414779d7391ca19128cc7b13794ea85af5ff835f59008258201c37df764f8261edce8678b197767668a91d544b2b203fb5d0cf9acc10366e7600a200818258200eabfa083d7969681d2fc8e825a5f79e1c40f03aeac46ecd94bf5c5790db1bc058409a029ddd3cdde65598bb712c640ea63eeebfee526ce49bd0983b4d1fdca858481ddf931bf0354552cc0a7d3365e2f03fdb457c0466cea8b371b645f9b6d0c2010582840001d8799fd8799f011a006336d5195e991b7ffffffffffade6bd8799f1a000539e7ff01ffff821a000b46e41a0a7f3ca4840003d87d80821a002dccfe1a28868be8f5f6";

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

        let local_body_str = String::from_utf8(local_body_bytes.to_vec())
            .expect("Failed to convert bytes to string");
        assert!(local_body_str.contains("MultiAsset cannot contain zeros"));
    }

    // Test: build `/tx/submit` success - tx is accepted by the node
    #[tokio::test]
    #[ntest::timeout(120_000)]
    async fn test_route_submit_success() {
        initialize_logging();
        let (app, _, _, _, _) = build_app().await.expect("Failed to build the application");
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

        assert_eq!(response.status(), StatusCode::OK);

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
    #[ntest::timeout(120_000)]
    async fn test_icebreakers_registrations() -> Result<(), BlockfrostError> {
        initialize_logging();

        let (app, _, _, icebreakers_api, api_prefix) = build_app_non_solitary()
            .await
            .expect("Failed to build the application");

        let ip_addr: IpAddr = "0.0.0.0".parse().unwrap();
        let address = SocketAddr::new(ip_addr, 3000);
        let listener = tokio::net::TcpListener::bind(address).await.unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let (ready_tx, ready_rx) = oneshot::channel();

        let spawn_task = tokio::spawn({
            let app = app.clone();

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

            if let Some(icebreakers_api) = icebreakers_api {
                let health_errors = Arc::new(Mutex::new(vec![]));

                let manager = IcebreakersManager::new(
                    icebreakers_api.clone(),
                    health_errors.clone(),
                    app.clone(),
                    api_prefix.clone(),
                );

                let response = manager.run_once().await?;
                let resp = response;
                let errors = health_errors.lock().await;

                info!("run_once response: {}", resp);

                assert!(
                    errors.is_empty(),
                    "Expected no WebSocket errors, but found: {:?}",
                    *errors
                );

                assert!(
                    resp.contains("Started"),
                    "Expected successful registration, but got: {resp}",
                );

                tokio::spawn(async move {
                    manager.run().await;
                });
            }
        }

        let _ = shutdown_tx.send(());

        spawn_task.await.unwrap().unwrap();

        Ok(())
    }
}
