use integration_tests::gateway::TestGateway;
use reqwest::Client;
use serde_json::json;

#[tokio::test]
async fn test_register_rate_limit_returns_429() {
    let gw = TestGateway::start_with_rate_limit(3).await;
    let client = Client::new();
    let url = format!("http://{}/register", gw.addr);

    let body = json!({
        "secret": "00000000",
        "api_prefix": uuid::Uuid::new_v4().to_string(),
    });

    // first 3 requests should succeed
    for i in 0..3 {
        let resp = client.post(&url).json(&body).send().await.unwrap();
        assert_ne!(
            resp.status().as_u16(),
            429,
            "Request {i} should not be rate limited"
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
        "secret": "00000000",
        "api_prefix": uuid::Uuid::new_v4().to_string(),
    });

    // requests should go through
    for _ in 0..5 {
        let resp = client.post(&url).json(&body).send().await.unwrap();
        assert_ne!(
            resp.status().as_u16(),
            429,
            "Should not be rate limited with generous limit"
        );
    }

    gw.stop().await;
}
