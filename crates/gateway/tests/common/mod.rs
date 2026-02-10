#![cfg(not(windows))]

use axum::{
    Extension, Router,
    routing::{any, get},
};
use blockfrost_gateway::load_balancer::{LoadBalancerState, api};
use std::net::SocketAddr;
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

pub async fn build_router(lb: LoadBalancerState) -> Router {
    Router::new()
        .route("/ws", get(api::websocket_route))
        .route("/:uuid", any(api::prefix_route_root))
        .route("/:uuid/*rest", any(api::prefix_route))
        .layer(Extension(lb))
}

pub async fn start_server(router: Router) -> (SocketAddr, JoinHandle<()>) {
    let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();

    let handle = tokio::spawn(async move {
        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .await
        .unwrap();
    });

    (addr, handle)
}
