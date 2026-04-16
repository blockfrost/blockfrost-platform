use axum::{
    Extension, Json, Router,
    extract::ConnectInfo,
    extract::rejection::JsonRejection,
    http::{HeaderMap, StatusCode},
    routing::{any, get, post},
};
use blockfrost_gateway::{
    load_balancer::{LoadBalancerState, api},
    rate_limit::{self, RegisterRateLimiter},
    types::AssetName,
};
use blockfrost_platform::{
    hydra_client, icebreakers::manager::IcebreakersManager, server::state::ApiPrefix,
};
use reqwest::{Client, Response};
use serde::Deserialize;
use serde_json::json;
use std::{net::SocketAddr, num::NonZeroU32, sync::Arc, time::Duration};
use tokio::{
    net::TcpListener,
    sync::{Mutex, oneshot},
    task::JoinHandle,
    time::Instant,
};

pub async fn build_router(lb: LoadBalancerState) -> Router {
    Router::new()
        .route("/ws", get(api::websocket_route))
        .route("/{uuid}", any(api::prefix_route_root))
        .route("/{uuid}/", any(api::prefix_route_root))
        .route("/{uuid}/{*rest}", any(api::prefix_route))
        .layer(Extension(lb))
}

pub async fn start_server(
    router: Router,
    addr: Option<SocketAddr>,
) -> (SocketAddr, oneshot::Sender<()>, JoinHandle<()>) {
    let bind_addr = addr.unwrap_or_else(|| "127.0.0.1:0".parse().unwrap());
    let listener = TcpListener::bind(bind_addr).await.unwrap();
    let addr = listener.local_addr().unwrap();

    let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();

    let handle = tokio::spawn(async move {
        axum::serve(
            listener,
            router.into_make_service_with_connect_info::<SocketAddr>(),
        )
        .with_graceful_shutdown(async {
            shutdown_rx.await.ok();
        })
        .await
        .unwrap();
    });

    (addr, shutdown_tx, handle)
}

/// A self-contained gateway with mock `/register` for E2E tests
pub struct TestGateway {
    pub addr: SocketAddr,
    pub lb: LoadBalancerState,
    pub rate_limiter: RegisterRateLimiter,
    shutdown_tx: Option<oneshot::Sender<()>>,
    server_handle: JoinHandle<()>,
}

pub const EXPECTED_SECRET: &str = "000666000";

impl TestGateway {
    /// Start a gateway on a random port with WS routes + mock /register.
    pub async fn start() -> Self {
        Self::start_on(None).await
    }

    /// Start a gateway bound to a specific address (for restart tests).
    pub async fn start_on(addr: Option<SocketAddr>) -> Self {
        let lb = LoadBalancerState::new(None).await;
        let rate_limiter = rate_limit::new_register_rate_limiter();
        let router = build_router(lb.clone())
            .await
            .route("/register", post(mock_register_handler))
            .layer(Extension(rate_limiter.clone()))
            .layer(Extension(lb.clone()));
        let (addr, shutdown_tx, server_handle) = start_server(router, addr).await;

        Self {
            addr,
            lb,
            rate_limiter,
            shutdown_tx: Some(shutdown_tx),
            server_handle,
        }
    }

    /// Start a gateway with a custom rate limit (for rate limit tests).
    pub async fn start_with_rate_limit(max_per_minute: u32) -> Self {
        let lb = LoadBalancerState::new(None).await;
        let quota = governor::Quota::per_minute(
            NonZeroU32::new(max_per_minute)
                .expect("TestGateway::start_with_rate_limit requires max_per_minute > 0"),
        );
        let rate_limiter: RegisterRateLimiter = Arc::new(governor::RateLimiter::keyed(quota));
        let router = build_router(lb.clone())
            .await
            .route("/register", post(mock_register_handler))
            .layer(Extension(rate_limiter.clone()))
            .layer(Extension(lb.clone()));
        let (addr, shutdown_tx, server_handle) = start_server(router, None).await;

        Self {
            addr,
            lb,
            rate_limiter,
            shutdown_tx: Some(shutdown_tx),
            server_handle,
        }
    }

    /// Shut down the gateway.
    pub async fn stop(mut self) {
        // Actively disconnect all WebSocket relays via their `do_finish` channels.
        // FIXME: we should better simulate Gateway getting killed.
        {
            let relays = self.lb.active_relays.lock().await;
            for (_, relay) in relays.iter() {
                let _ = relay.do_finish.send("test gateway stopping".into()).await;
            }
        }
        // Signal server shutdown and wait briefly, then abort.
        if let Some(tx) = self.shutdown_tx.take() {
            let _ = tx.send(());
        }
        let timeout =
            tokio::time::timeout(std::time::Duration::from_secs(2), &mut self.server_handle);
        if timeout.await.is_err() {
            self.server_handle.abort();
            let _ = self.server_handle.await;
        }
    }
}

#[derive(Deserialize)]
struct RegisterPayload {
    secret: String,
    api_prefix: String,
}

async fn mock_register_handler(
    Extension(lb): Extension<LoadBalancerState>,
    Extension(rate_limiter): Extension<RegisterRateLimiter>,
    ConnectInfo(addr): ConnectInfo<SocketAddr>,
    headers: HeaderMap,
    payload: Result<Json<RegisterPayload>, JsonRejection>,
) -> Result<Json<serde_json::Value>, (StatusCode, Json<serde_json::Value>)> {
    // rate limit by ip
    if rate_limiter.check_key(&addr.ip()).is_err() {
        return Err((
            StatusCode::TOO_MANY_REQUESTS,
            Json(json!({
                "status": "failed",
                "reason": "rate_limited",
                "details": "Too many registration requests. Please try again later."
            })),
        ));
    }
    let Json(payload) = payload.map_err(|e| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "reason": "Invalid payload",
                "details": e.to_string()
            })),
        )
    })?;

    if payload.secret != EXPECTED_SECRET {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(json!({
                "reason": "Invalid secret",
                "details": "The provided secret does not match"
            })),
        ));
    }

    let api_prefix: uuid::Uuid = payload.api_prefix.parse().map_err(|e: uuid::Error| {
        (
            StatusCode::BAD_REQUEST,
            Json(json!({
                "reason": "Invalid api_prefix",
                "details": e.to_string()
            })),
        )
    })?;

    let token = lb
        .new_access_token(AssetName("test".into()), api_prefix, "reward_addr_test")
        .await;

    let host = headers
        .get("Host")
        .and_then(|h| h.to_str().ok())
        .unwrap_or("localhost");

    Ok(Json(json!({
        "route": payload.api_prefix,
        "status": "registered",
        "load_balancers": [{
            "uri": format!("//{host}/ws"),
            "access_token": token.0
        }]
    })))
}

/// Poll until the gateway is serving requests through the WebSocket relay.
///
/// The gateway returns 502 when no relay is connected for the UUID prefix.
pub async fn wait_for_ready(client: &Client, url: &str, timeout: Duration) -> Response {
    let deadline = Instant::now() + timeout;
    loop {
        if let Ok(resp) = client.get(url).send().await {
            let status = resp.status();
            // 502 = no WebSocket relay connected; 404 = UUID not registered yet
            if status != StatusCode::BAD_GATEWAY
                && status != StatusCode::NOT_FOUND
                && status != StatusCode::SERVICE_UNAVAILABLE
            {
                return resp;
            }
        }
        assert!(Instant::now() < deadline, "Timed out waiting for relay");
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Common setup: start a `TestGateway` + Platform, wire through `IcebreakersManager`.
///
/// Returns `(gateway, client, base_url_with_prefix)` for making requests.
pub async fn setup() -> (TestGateway, Client, String, ApiPrefix) {
    crate::initialize_logging();

    let gw = TestGateway::start().await;
    let gateway_url = format!("http://{}", gw.addr);

    let (app, _, _, icebreakers_api, api_prefix) =
        crate::platform::build_app_non_solitary(Some(gateway_url))
            .await
            .expect("Failed to build the application");

    let icebreakers_api = icebreakers_api.expect("icebreakers_api should be Some");
    let health_errors = Arc::new(Mutex::new(vec![]));

    let manager = IcebreakersManager::new(icebreakers_api, health_errors, app, api_prefix.clone());

    let (kex_req_tx, kex_req_rx) =
        tokio::sync::mpsc::channel::<hydra_client::KeyExchangeRequest>(1);
    let (kex_resp_tx, _kex_resp_rx) =
        tokio::sync::mpsc::channel::<hydra_client::KeyExchangeResponse>(1);
    let (terminate_tx, _terminate_rx) =
        tokio::sync::mpsc::channel::<hydra_client::TerminateRequest>(1);
    drop(kex_req_tx); // not used in tests
    manager.run((kex_req_rx, kex_resp_tx, terminate_tx)).await;

    let client = Client::new();
    let base = format!("http://{}{}", gw.addr, api_prefix);

    // Wait for the relay to be ready and for the Platform root to return 200.
    wait_for_ready(&client, &format!("{base}/"), Duration::from_secs(30)).await;

    (gw, client, base, api_prefix)
}
