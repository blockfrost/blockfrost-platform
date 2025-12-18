mod common;
use common::*;

use base64::Engine;
use blockfrost_gateway::{
    blockfrost::AssetName,
    load_balancer::{JsonResponse, LoadBalancerMessage, LoadBalancerState, RelayMessage},
};
use futures::{SinkExt, StreamExt};
use std::vec;
use tungstenite::{Message, handshake::client::generate_key};
use uuid::Uuid;

#[tokio::test]
async fn test_websocket_connection_invalid_token() {
    let lb = LoadBalancerState::new(None).await;

    let router = build_router(lb.clone()).await;
    let (addr, server_handle) = start_server(router).await;

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
    let lb = LoadBalancerState::new(None).await;

    let name = AssetName("test-asset".to_string());
    let prefix = Uuid::new_v4();
    let token = lb.new_access_token(name.clone(), prefix).await;

    let router = build_router(lb.clone()).await;
    let (addr, server_handle) = start_server(router).await;

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
                            body_base64: base64::engine::general_purpose::STANDARD
                                .encode(b"test response"),
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
