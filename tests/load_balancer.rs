use base64::Engine;
use blockfrost_icebreakers_api::{
    blockfrost::AssetName,
    errors::APIError,
    load_balancer::{
        api, random_token, AccessTokenState, JsonResponse, LoadBalancerMessage, LoadBalancerState, RelayMessage,
    },
};
use futures::{SinkExt, StreamExt};
use std::{net::SocketAddr, vec};
use tungstenite::{handshake::client::generate_key, Message};
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

#[tokio::test]
async fn test_websocket_connection_invalid_token() {
    let lb = LoadBalancerState::new().await;
    let app = axum::Router::new()
        .route("/ws", axum::routing::get(api::websocket_route))
        .layer(axum::Extension(lb.clone()));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let server_handle = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    let url = format!("ws://{}", addr);

    let request = hyper::Request::builder()
        .uri(&url)
        .header("Authorization", "Bearer invalid")
        .body(())
        .unwrap();

    let connect_result = tokio_tungstenite::connect_async(request).await;

    assert!(connect_result.is_err());

    server_handle.abort();
}

#[tokio::test]
async fn test_websocket_request_response_flow() {
    let lb = LoadBalancerState::new().await;

    let name = AssetName("test-asset".to_string());
    let prefix = Uuid::new_v4();
    let token = lb.new_access_token(name.clone(), prefix).await;

    let app = axum::Router::new()
        .route("/ws", axum::routing::get(api::websocket_route))
        .route("/:uuid", axum::routing::any(api::prefix_route_root))
        .route("/:uuid/*rest", axum::routing::any(api::prefix_route))
        .layer(axum::Extension(lb.clone()));

    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let server_handle = tokio::spawn(async move {
        axum::serve(
            listener,
            app.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    let ws_url = format!("ws://{}/ws", addr);
    let http_url = format!("http://{}", addr);

    let request = hyper::Request::builder()
        .uri(&ws_url)
        .header("Host", addr.to_string())
        .header("Connection", "Upgrade")
        .header("Upgrade", "websocket")
        .header("Sec-WebSocket-Version", "13")
        .header("Sec-WebSocket-Key", generate_key())
        .header("Authorization", format!("Bearer {}", token.0))
        .body(())
        .unwrap();

    let (ws_stream, _) = tokio_tungstenite::connect_async(request)
        .await
        .expect("failed to connect");

    let (mut relay_tx, mut relay_rx) = ws_stream.split();

    let relay_handle = tokio::spawn(async move {
        while let Some(Ok(msg)) = relay_rx.next().await {
            if let Message::Text(text) = msg {
                let lb_msg = serde_json::from_str::<LoadBalancerMessage>(&text).expect("parse msg");
                match lb_msg {
                    LoadBalancerMessage::Request(json_req) => {
                        let response = JsonResponse {
                            id: json_req.id,
                            code: 200,
                            header: vec![],
                            body_base64: base64::engine::general_purpose::STANDARD.encode(b"test response"),
                        };
                        let relay_msg = RelayMessage::Response(response);

                        relay_tx
                            .send(Message::Text(serde_json::to_string(&relay_msg).unwrap()))
                            .await
                            .unwrap();
                    },
                    LoadBalancerMessage::Ping(id) => {
                        let pong = RelayMessage::Pong(id);
                        relay_tx
                            .send(Message::Text(serde_json::to_string(&pong).unwrap()))
                            .await
                            .unwrap();
                    },
                    _ => {},
                }
            }
        }
    });

    tokio::time::sleep(std::time::Duration::from_millis(500)).await;

    let client = reqwest::Client::new();
    let res = client
        .get(format!("{}/{}/test", http_url, prefix))
        .send()
        .await
        .expect("http request failed");

    assert_eq!(res.status(), 200);
    assert_eq!(res.text().await.unwrap(), "test response");

    relay_handle.abort();
    server_handle.abort();
}
