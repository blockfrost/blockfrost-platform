use std::{sync::Arc, time::Duration};

use futures::future::join_all;
use integration_tests::{
    gateway::{self, TestGateway},
    initialize_logging,
    platform::test_config,
};

use blockfrost_platform::config::IcebreakersConfig;
use blockfrost_platform::{hydra_client, icebreakers::manager::IcebreakersManager, server::build};
use reqwest::StatusCode;
use tokio::sync::Mutex;

#[tokio::test]
#[ntest::timeout(120_000)]
async fn test_ws_connection_and_basic_request() {
    let (_gw, client, base, _prefix) = gateway::setup().await;

    let resp = client
        .get(format!("{base}/health"))
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body.get("is_healthy").is_some(),
        "Expected health response: {body}"
    );
}

#[tokio::test]
#[ntest::timeout(120_000)]
async fn test_ws_multiple_sequential_requests() {
    let (_gw, client, base, _prefix) = gateway::setup().await;

    for i in 0..5 {
        let url = if i % 2 == 0 {
            format!("{base}/health")
        } else {
            format!("{base}/")
        };

        let resp = client.get(&url).send().await.expect("request failed");
        assert_eq!(
            resp.status(),
            StatusCode::OK,
            "Request {i} to {url} failed with status {}",
            resp.status()
        );
        let _body: serde_json::Value = resp.json().await.unwrap();
    }
}

#[tokio::test]
#[ntest::timeout(120_000)]
async fn test_ws_concurrent_requests() {
    let (_gw, client, base, _prefix) = gateway::setup().await;

    let futs: Vec<_> = (0..10)
        .map(|i| {
            let client = client.clone();
            let url = format!("{base}/health");
            tokio::spawn(async move {
                let resp = client.get(&url).send().await.expect("request failed");
                assert_eq!(
                    resp.status(),
                    StatusCode::OK,
                    "Concurrent request {i} failed"
                );
            })
        })
        .collect();

    let results = join_all(futs).await;
    for r in results {
        r.expect("task panicked");
    }
}

#[tokio::test]
#[ntest::timeout(120_000)]
async fn test_ws_post_request_with_body() {
    let (_gw, client, base, _prefix) = gateway::setup().await;

    // Send invalid CBOR to `tx/submit`. The Platform should receive it and
    // return a structured error, proving the `base64` body round-trip works:
    let resp = client
        .post(format!("{base}/tx/submit"))
        .header("Content-Type", "application/cbor")
        .body(b"not-valid-cbor".to_vec())
        .send()
        .await
        .expect("request failed");

    assert_eq!(resp.status(), StatusCode::BAD_REQUEST);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body.get("status_code").and_then(|v| v.as_u64()), Some(400));
}

#[tokio::test]
#[ntest::timeout(120_000)]
async fn test_ws_invalid_credentials_rejected() {
    initialize_logging();

    let gw = TestGateway::start().await;
    let gateway_url = format!("http://{}", gw.addr);

    // Wrong secret:
    let icebreakers_config = IcebreakersConfig {
        secret: "wrong-secret".to_string(),
        reward_address: "addr_test1qrwlr6uuu2s4v850z45ezjrtj7rnld5kjxgvhjvamjecze3pmjcr2aq4yc35znkn2nfd3agwxy8n7tnaze7tyrjh2snspw9f3g".to_string(),
        gateway_url: Some(gateway_url),
    };
    let config = test_config(Some(icebreakers_config));

    let (app, _, _, icebreakers_api, api_prefix, _) =
        build(config).await.expect("Failed to build app");

    let icebreakers_api = icebreakers_api.expect("icebreakers_api should be Some");
    let health_errors = Arc::new(Mutex::new(vec![]));

    let manager = IcebreakersManager::new(icebreakers_api, health_errors.clone(), app, api_prefix);

    let (kex_req_tx, kex_req_rx) =
        tokio::sync::mpsc::channel::<hydra_client::KeyExchangeRequest>(1);
    let (kex_resp_tx, _kex_resp_rx) =
        tokio::sync::mpsc::channel::<hydra_client::KeyExchangeResponse>(1);
    let (terminate_tx, _terminate_rx) =
        tokio::sync::mpsc::channel::<hydra_client::TerminateRequest>(1);
    drop(kex_req_tx);
    manager.run((kex_req_rx, kex_resp_tx, terminate_tx)).await;

    // Wait long enough for the first registration attempt to fail. The manager
    // retries after 10 s on error, so 2 s is enough for the first attempt:
    tokio::time::sleep(Duration::from_secs(2)).await;

    let errors = health_errors.lock().await;
    assert!(
        !errors.is_empty(),
        "Expected health_errors to contain a registration error"
    );
    let error_msg = format!("{}", errors[0]);
    assert!(
        error_msg.contains("Invalid secret"),
        "Expected 'Invalid secret' in error, got: {error_msg}"
    );
}

#[tokio::test]
#[ntest::timeout(120_000)]
async fn test_ws_reconnection_after_gateway_restart() {
    let (gw, client, base, _prefix) = gateway::setup().await;
    let addr = gw.addr;

    // Verify initial request works:
    let resp = client
        .get(format!("{base}/health"))
        .send()
        .await
        .expect("request failed");
    assert_eq!(resp.status(), StatusCode::OK);

    // Stop the gateway:
    gw.stop().await;

    // Restart gateway on the same address:
    let _gw2 = TestGateway::start_on(Some(addr)).await;

    // Wait for the manager to re-register and reconnect (may take up to ~12 s
    // in the worst case: 1 s disconnect delay + 10 s failed-register retry):
    let resp =
        gateway::wait_for_ready(&client, &format!("{base}/health"), Duration::from_secs(30)).await;
    assert_eq!(resp.status(), StatusCode::OK);

    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body.get("is_healthy").is_some(),
        "Expected health response after reconnection: {body}"
    );
}

#[tokio::test]
#[ntest::timeout(120_000)]
async fn test_ws_cardano_api_endpoints() {
    let (_gw, client, base, _prefix) = gateway::setup().await;

    // Test root endpoint (returns Platform metadata from `HealthMonitor`):
    let resp = client
        .get(format!("{base}/"))
        .send()
        .await
        .expect("request failed");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body.get("name").is_some(),
        "Expected root API metadata with 'name': {body}"
    );

    // Test genesis endpoint (reads from genesis config, no data_node needed):
    let resp = client
        .get(format!("{base}/genesis"))
        .send()
        .await
        .expect("request failed");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body.get("network_magic").is_some(),
        "Expected genesis data with 'network_magic': {body}"
    );

    // Test health/clock endpoint (returns server timestamp):
    let resp = client
        .get(format!("{base}/health/clock"))
        .send()
        .await
        .expect("request failed");
    assert_eq!(resp.status(), StatusCode::OK);
    let body: serde_json::Value = resp.json().await.unwrap();
    assert!(
        body.get("server_time").is_some(),
        "Expected clock data with 'server_time': {body}"
    );
}
