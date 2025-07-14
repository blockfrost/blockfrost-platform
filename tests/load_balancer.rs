use blockfrost_icebreakers_api::{blockfrost::AssetName, load_balancer::LoadBalancerState};
use uuid::Uuid;

#[tokio::test]
async fn test_new_creates_empty_state() {
    let lb = LoadBalancerState::new().await;

    let tokens = lb.access_tokens.lock().await;
    assert!(tokens.is_empty());

    let relays = lb.active_relays.lock().await;
    assert!(relays.is_empty());
}

#[tokio::test]
async fn test_new_access_token_register() {
    let lb = LoadBalancerState::new().await;
    let name = AssetName("x-asset-x".to_string());
    let prefix = Uuid::new_v4();

    let token = lb.new_access_token(name.clone(), prefix).await;
    let state = lb.register(&token.0).await.expect("should register");

    assert_eq!(state.name, name);
    assert_eq!(state.api_prefix, prefix);
}
