use integration_tests::gateway::{EXPECTED_SECRET, TestGateway};
use reqwest::Client;
use serde_json::json;

#[tokio::test]
async fn test_register_rate_limit_returns_429() {
    let gw = TestGateway::start_with_rate_limit(3).await;
    let client = Client::new();
    let url = format!("http://{}/register", gw.addr);

    let body = json!({
        "secret": EXPECTED_SECRET,
        "api_prefix": uuid::Uuid::new_v4().to_string(),
    });

    // first 3 requests should succeed
    for i in 0..3 {
        let resp = client.post(&url).json(&body).send().await.unwrap();
        let status = resp.status();
        let body_json: serde_json::Value = resp.json().await.unwrap();
        assert!(
            status.is_success(),
            "Request {i} should succeed, got {status}: {body_json}"
        );
        assert_eq!(
            body_json["status"], "registered",
            "Request {i} should register, got {body_json}"
        );
    }

    // 4th rate limited
    let resp = client.post(&url).json(&body).send().await.unwrap();
    assert_eq!(
        resp.status().as_u16(),
        429,
        "Request should be rate limited after burst is exhausted"
    );

    let body_json: serde_json::Value = resp.json().await.unwrap();
    assert_eq!(body_json["reason"], "rate_limited");

    gw.stop().await;
}

#[tokio::test]
async fn test_register_within_rate_limit_succeeds() {
    let gw = TestGateway::start_with_rate_limit(100).await;
    let client = Client::new();
    let url = format!("http://{}/register", gw.addr);

    let body = json!({
        "secret": EXPECTED_SECRET,
        "api_prefix": uuid::Uuid::new_v4().to_string(),
    });

    // requests should go through
    for i in 0..5 {
        let resp = client.post(&url).json(&body).send().await.unwrap();
        let status = resp.status();
        let body_json: serde_json::Value = resp.json().await.unwrap();
        assert!(
            status.is_success(),
            "Request {i} should succeed under generous limit, got {status}: {body_json}"
        );
        assert_eq!(body_json["status"], "registered");
    }

    gw.stop().await;
}
