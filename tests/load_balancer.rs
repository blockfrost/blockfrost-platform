use blockfrost_icebreakers_api::{
    blockfrost::AssetName,
    errors::APIError,
    load_balancer::{random_token, AccessTokenState, LoadBalancerState},
};
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

    // token should be removed after register
    let tokens = lb.access_tokens.lock().await;
    assert!(tokens.is_empty());
}

#[tokio::test]
async fn test_register_invalid_token() {
    let lb = LoadBalancerState::new().await;
    let res = lb.register("invalid").await;
    assert!(matches!(res, Err(APIError::Unauthorized())));
}

#[tokio::test]
async fn test_register_expired_token() {
    let lb = LoadBalancerState::new().await;
    let name = AssetName("x-asset-x".to_string());
    let prefix = Uuid::new_v4();
    let token = random_token();
    let expires = std::time::Instant::now() - std::time::Duration::from_secs(1);

    lb.access_tokens.lock().await.insert(
        token.clone(),
        AccessTokenState {
            name,
            api_prefix: prefix,
            expires,
        },
    );

    let res = lb.register(&token.0).await;

    assert!(matches!(res, Err(APIError::Unauthorized())));
}

#[tokio::test]
async fn test_clean_up_expired_tokens_logic() {
    let lb = LoadBalancerState::new().await;
    let name = AssetName("x-asset-x".to_string());
    let prefix = Uuid::new_v4();

    // insert expired token
    let token_expired = random_token();
    let expires_expired = std::time::Instant::now() - std::time::Duration::from_secs(1);

    lb.access_tokens.lock().await.insert(
        token_expired.clone(),
        AccessTokenState {
            name: name.clone(),
            api_prefix: prefix,
            expires: expires_expired,
        },
    );

    // insert valid token
    let token_valid = random_token();
    let expires_valid = std::time::Instant::now() + std::time::Duration::from_secs(300);
    lb.access_tokens.lock().await.insert(
        token_valid.clone(),
        AccessTokenState {
            name,
            api_prefix: prefix,
            expires: expires_valid,
        },
    );

    // cleanup
    let now = std::time::Instant::now();
    lb.access_tokens.lock().await.retain(|_, state| state.expires > now);

    let tokens = lb.access_tokens.lock().await;

    assert_eq!(tokens.len(), 1);
    assert!(tokens.contains_key(&token_valid));
    assert!(!tokens.contains_key(&token_expired));
}
