use anyhow::{Result, anyhow};
use axum::extract::Request;
use axum::response::IntoResponse;
use axum::Router;
use axum::routing::any;
use crate::find_libexec;
use crate::hydra;
use crate::load_balancer::{JsonHeader, JsonRequest, JsonResponse, JsonRequestMethod, RequestId};
use crate::types::Network;
use clap::Parser;
use futures_util::{SinkExt, StreamExt};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc, oneshot};
use tracing::{error, info, warn};

const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
const WS_PING_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(13);
const MAX_BODY_BYTES: usize = 1024 * 1024;

#[derive(Parser, Debug)]
#[command(name = "blockfrost-sdk-bridge", // otherwise it’s `blockfrost-gateway`
          bin_name = "blockfrost-sdk-bridge",
          version, about, long_about = None)]
struct Cli {
    /// WebSocket URL of the gateway (ws/wss or http/https).
    #[arg(long, default_value = "https://icebreakers1.platform.blockfrost.io")]
    pub gateway_ws_url: String,

    /// Local HTTP bind address for the bridge.
    #[arg(long, default_value = "127.0.0.1:3001")]
    pub listen_address: String,

    /// Cardano network.
    #[arg(long, value_enum)]
    pub network: Network,

    /// Cardano node socket path.
    #[arg(long)]
    pub node_socket_path: PathBuf,

    /// L1 key for committing and fees.
    #[arg(long)]
    pub cardano_signing_key: PathBuf,

    /// ADA to commit when opening a Hydra head.
    #[arg(long, default_value_t = 3.0)]
    pub commit_ada: f64,

    /// Lovelace per request.
    #[arg(long, default_value_t = 100_000)]
    pub lovelace_per_request: u64,

    /// Requests per microtransaction.
    #[arg(long, default_value_t = 10)]
    pub requests_per_microtransaction: u64,

    /// Microtransactions per fanout.
    #[arg(long, default_value_t = 2)]
    pub microtransactions_per_fanout: u64,
}

#[derive(Serialize, Deserialize, Debug)]
enum BridgeMessage {
    Request(JsonRequest),
    HydraKExRequest(hydra::client::KeyExchangeRequest),
    HydraTunnel(hydra::tunnel2::TunnelMsg),
    Ping(u64),
    Pong(u64),
}

#[derive(Serialize, Deserialize, Debug)]
enum GatewayMessage {
    Response(JsonResponse),
    HydraKExResponse(hydra::client::KeyExchangeResponse),
    HydraTunnel(hydra::tunnel2::TunnelMsg),
    Ping(u64),
    Pong(u64),
    Error { code: u64, msg: String },
}

#[derive(Clone)]
struct BridgeState {
    ws_tx: mpsc::Sender<tokio_tungstenite::tungstenite::Message>,
    inflight: Arc<Mutex<HashMap<RequestId, oneshot::Sender<JsonResponse>>>>,
    hydra: hydra::client::HydraController,
    hydra_head_open: Arc<Mutex<bool>>,
    gateway_address: Arc<Mutex<Option<String>>>,
    credits: Arc<Mutex<u64>>,
    payment_params: Arc<Mutex<PaymentParams>>,
    node_socket_path: PathBuf,
    network: Network,
}

#[derive(Clone, Debug)]
struct PaymentParams {
    lovelace_per_request: u64,
    requests_per_microtransaction: u64,
    microtransactions_per_fanout: u64,
}

impl PaymentParams {
    fn from_cli(cli: &Cli) -> Self {
        Self {
            lovelace_per_request: cli.lovelace_per_request,
            requests_per_microtransaction: cli.requests_per_microtransaction,
            microtransactions_per_fanout: cli.microtransactions_per_fanout,
        }
    }

    fn update_from_kex(&mut self, resp: &hydra::client::KeyExchangeResponse) {
        if resp.lovelace_per_request > 0 {
            self.lovelace_per_request = resp.lovelace_per_request;
        }
        if resp.requests_per_microtransaction > 0 {
            self.requests_per_microtransaction = resp.requests_per_microtransaction;
        }
        if resp.microtransactions_per_fanout > 0 {
            self.microtransactions_per_fanout = resp.microtransactions_per_fanout;
        }
    }
}

#[tokio::main]
pub async fn run() -> Result<()> {
    tracing_subscriber::fmt().with_target(false).compact().init();

    let cli = Cli::parse();
    let gateway_ws_url = normalize_gateway_ws_url(&cli.gateway_ws_url);

    let (kex_req_tx, mut kex_req_rx) = mpsc::channel(32);
    let (kex_resp_tx, kex_resp_rx) = mpsc::channel(32);
    let (terminate_tx, terminate_rx) = mpsc::channel(4);

    let hydra_config = hydra::client::HydraClientConfig {
        cardano_signing_key: cli.cardano_signing_key.clone(),
        commit_ada: cli.commit_ada,
        lovelace_per_request: cli.lovelace_per_request,
        requests_per_microtransaction: cli.requests_per_microtransaction,
        microtransactions_per_fanout: cli.microtransactions_per_fanout,
    };

    let hydra = hydra::client::HydraController::spawn(
        hydra_config,
        cli.network.clone(),
        cli.node_socket_path.to_string_lossy().to_string(),
        "".to_string(),
        Arc::new(Mutex::new(Vec::new())),
        kex_req_tx.clone(),
        kex_resp_rx,
        terminate_rx,
    )
    .await?;

    let (ws_stream, _response) = tokio_tungstenite::connect_async(&gateway_ws_url)
        .await
        .map_err(|e| anyhow!("failed to connect to {gateway_ws_url}: {e}"))?;
    info!("connected to {gateway_ws_url}");

    let (mut ws_write, mut ws_read) = ws_stream.split();
    let (ws_tx, mut ws_rx) = mpsc::channel::<tokio_tungstenite::tungstenite::Message>(64);

    let inflight = Arc::new(Mutex::new(HashMap::new()));
    let gateway_address = Arc::new(Mutex::new(None));
    let hydra_head_open = Arc::new(Mutex::new(false));
    let credits = Arc::new(Mutex::new(0));
    let payment_params = Arc::new(Mutex::new(PaymentParams::from_cli(&cli)));

    let bridge_state = BridgeState {
        ws_tx: ws_tx.clone(),
        inflight: inflight.clone(),
        hydra: hydra.clone(),
        hydra_head_open: hydra_head_open.clone(),
        gateway_address: gateway_address.clone(),
        credits: credits.clone(),
        payment_params: payment_params.clone(),
        node_socket_path: cli.node_socket_path.clone(),
        network: cli.network.clone(),
    };

    let writer_task = tokio::spawn(async move {
        while let Some(msg) = ws_rx.recv().await {
            if let Err(err) = ws_write.send(msg).await {
                error!("ws send error: {err}");
                break;
            }
        }
    });

    let ping_tx = ws_tx.clone();
    tokio::spawn(async move {
        let mut ping_id = 0u64;
        loop {
            tokio::time::sleep(WS_PING_TIMEOUT).await;
            ping_id += 1;
            if send_ws_msg(&ping_tx, &BridgeMessage::Ping(ping_id)).await.is_err() {
                break;
            }
        }
    });

    let kex_ws_tx = ws_tx.clone();
    tokio::spawn(async move {
        while let Some(req) = kex_req_rx.recv().await {
            if send_ws_msg(&kex_ws_tx, &BridgeMessage::HydraKExRequest(req))
                .await
                .is_err()
            {
                break;
            }
        }
    });

    let inflight_ = inflight.clone();
    let hydra_head_open_ = hydra_head_open.clone();
    tokio::spawn(async move {
        let api_port = hydra.wait_api_port().await;
        if api_port == 0 {
            return;
        }
        loop {
            match hydra::client::verifications::fetch_head_tag(api_port).await {
                Ok(tag) => {
                    let mut guard = hydra_head_open_.lock().await;
                    *guard = tag == "Open";
                },
                Err(err) => {
                    warn!("hydra head status error: {err}");
                    let mut guard = hydra_head_open_.lock().await;
                    *guard = false;
                },
            }
            if inflight_.lock().await.is_empty() {
                tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            } else {
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
        }
    });

    let bridge_state_for_ws = bridge_state.clone();
    let tunnel_cancellation = tokio_util::sync::CancellationToken::new();
    let mut tunnel_controller: Option<hydra::tunnel2::Tunnel> = None;

    let reader_task = tokio::spawn(async move {
        while let Some(msg) = ws_read.next().await {
            match msg {
                Ok(tokio_tungstenite::tungstenite::Message::Text(text)) => {
                    match serde_json::from_str::<GatewayMessage>(&text) {
                        Ok(GatewayMessage::Response(response)) => {
                            let mut guard = bridge_state_for_ws.inflight.lock().await;
                            if let Some(tx) = guard.remove(&response.id) {
                                let _ = tx.send(response);
                            }
                        },
                        Ok(GatewayMessage::HydraKExResponse(resp)) => {
                            if resp.machine_id != hydra::client::verifications::hashed_machine_id() {
                                let (tunnel_ctl, mut tunnel_rx) = hydra::tunnel2::Tunnel::new(
                                    hydra::tunnel2::TunnelConfig {
                                        expose_port: resp.proposed_platform_h2h_port,
                                        id_prefix_bit: true,
                                        ..(hydra::tunnel2::TunnelConfig::default())
                                    },
                                    tunnel_cancellation.clone(),
                                );

                                tunnel_ctl.spawn_listener(resp.gateway_h2h_port).await.expect("tunnel listener should start");

                                let ws_tx = bridge_state_for_ws.ws_tx.clone();
                                tokio::spawn(async move {
                                    while let Some(tun_msg) = tunnel_rx.recv().await {
                                        if send_ws_msg(&ws_tx, &BridgeMessage::HydraTunnel(tun_msg))
                                            .await
                                            .is_err()
                                        {
                                            break;
                                        }
                                    }
                                });

                                tunnel_controller = Some(tunnel_ctl);
                            }

                            {
                                let mut params = bridge_state_for_ws.payment_params.lock().await;
                                params.update_from_kex(&resp);
                            }

                            let addr = match derive_enterprise_address_from_vkey_json(
                                &resp.gateway_cardano_vkey,
                                &bridge_state_for_ws.network,
                                &bridge_state_for_ws.node_socket_path,
                            )
                            .await
                            {
                                Ok(addr) => addr,
                                Err(err) => {
                                    error!("failed to derive gateway payment address: {err}");
                                    continue;
                                }
                            };
                            info!("gateway payment address: {addr}");
                            *bridge_state_for_ws.gateway_address.lock().await = Some(addr);

                            let _ = kex_resp_tx.send(resp).await;
                        },
                        Ok(GatewayMessage::HydraTunnel(tun_msg)) => {
                            if let Some(tunnel_ctl) = &tunnel_controller {
                                if let Err(err) = tunnel_ctl.on_msg(tun_msg).await {
                                    error!("hydra-tunnel error: {err}");
                                }
                            }
                        },
                        Ok(GatewayMessage::Ping(ping_id)) => {
                            let _ = send_ws_msg(&bridge_state_for_ws.ws_tx, &BridgeMessage::Pong(ping_id)).await;
                        },
                        Ok(GatewayMessage::Pong(_)) => {},
                        Ok(GatewayMessage::Error { code, msg }) => {
                            warn!("gateway error: {code}: {msg}");
                        },
                        Err(err) => warn!("unparsable gateway message: {err}")
                    }
                },
                Ok(tokio_tungstenite::tungstenite::Message::Close(frame)) => {
                    warn!("gateway closed ws: {:?}", frame);
                    break;
                },
                Ok(_) => {},
                Err(err) => {
                    warn!("ws read error: {err}");
                    break;
                },
            }
        }

        tunnel_cancellation.cancel();
        let _ = terminate_tx.send(hydra::client::TerminateRequest).await;
    });

    let app = Router::new()
        .fallback(any(proxy_route))
        .layer(axum::Extension(bridge_state));

    let listener = tokio::net::TcpListener::bind(&cli.listen_address).await?;
    info!("bridge listening on http://{}", cli.listen_address);

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap_or_else(|e| {
        eprintln!("Server error: {e}");
        std::process::exit(1);
    });

    let _ = reader_task.await;
    let _ = writer_task.await;
    Ok(())
}

async fn proxy_route(
    axum::Extension(state): axum::Extension<BridgeState>,
    req: Request,
) -> impl IntoResponse {
    match handle_request(state, req).await {
        Ok(resp) => resp,
        Err((code, msg)) => (code, msg).into_response(),
    }
}

async fn handle_request(
    state: BridgeState,
    req: Request,
) -> Result<hyper::Response<axum::body::Body>, (hyper::StatusCode, String)> {
    let head_open = *state.hydra_head_open.lock().await;
    if !head_open {
        return Err((
            hyper::StatusCode::SERVICE_UNAVAILABLE,
            "Hydra head is not open yet".to_string(),
        ));
    }

    ensure_credit(&state).await.map_err(|msg| {
        (
            hyper::StatusCode::PAYMENT_REQUIRED,
            msg,
        )
    })?;

    let json_req = request_to_json(req).await?;
    let request_id = json_req.id.clone();

    let (tx, rx) = oneshot::channel();
    state.inflight.lock().await.insert(request_id.clone(), tx);

    send_ws_msg(&state.ws_tx, &BridgeMessage::Request(json_req))
        .await
        .map_err(|err| {
            (
                hyper::StatusCode::BAD_GATEWAY,
                format!("failed to send request over ws: {err}"),
            )
        })?;

    let response = tokio::time::timeout(REQUEST_TIMEOUT, rx)
        .await
        .map_err(|_| {
            (
                hyper::StatusCode::GATEWAY_TIMEOUT,
                format!("Timed out after {REQUEST_TIMEOUT:?} waiting for response"),
            )
        })
        .and_then(|res| {
            res.map_err(|_| {
                (
                    hyper::StatusCode::BAD_GATEWAY,
                    "gateway dropped response".to_string(),
                )
            })
        });

    let response = match response {
        Ok(resp) => resp,
        Err(err) => {
            state.inflight.lock().await.remove(&request_id);
            return Err(err);
        },
    };

    let http_response = json_to_response(response)
        .await
        .map_err(|e| (hyper::StatusCode::BAD_GATEWAY, e))?;

    state.hydra.account_one_request().await;

    Ok(http_response)
}

async fn request_to_json(
    request: hyper::Request<axum::body::Body>,
) -> Result<JsonRequest, (hyper::StatusCode, String)> {
    use axum::http::{Method, StatusCode};

    let method = match request.method() {
        &Method::GET => Ok(JsonRequestMethod::GET),
        &Method::POST => Ok(JsonRequestMethod::POST),
        other => Err((
            StatusCode::BAD_REQUEST,
            format!("unhandled request method: {other}"),
        )),
    }?;

    let path = request
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str().to_string())
        .unwrap_or_else(|| request.uri().path().to_string());

    let header: Vec<JsonHeader> = request
        .headers()
        .iter()
        .flat_map(|(name, value)| {
            value.to_str().ok().map(|value| JsonHeader {
                name: name.to_string(),
                value: value.to_string(),
            })
        })
        .collect();

    let body = request.into_body();
    let body_bytes = axum::body::to_bytes(body, MAX_BODY_BYTES)
        .await
        .map_err(|err| {
            (
                StatusCode::BAD_REQUEST,
                format!("failed to read body bytes: {err}"),
            )
        })?;

    use base64::{Engine as _, engine::general_purpose};
    let body_base64 = general_purpose::STANDARD.encode(body_bytes);

    Ok(JsonRequest {
        id: RequestId::new(),
        path,
        method,
        body_base64,
        header,
    })
}

async fn json_to_response(
    json: JsonResponse,
) -> Result<hyper::Response<axum::body::Body>, String> {
    use axum::body::Body;
    use hyper::Response;
    use hyper::StatusCode;

    let body: Body = {
        if json.body_base64.is_empty() {
            Body::empty()
        } else {
            use base64::{Engine as _, engine::general_purpose};
            let body_bytes: Vec<u8> =
                general_purpose::STANDARD
                    .decode(json.body_base64)
                    .map_err(|err| {
                        format!("Invalid base64 encoding of response body_base64: {err}")
                    })?;
            Body::from(body_bytes)
        }
    };

    let mut rv = Response::builder().status(StatusCode::from_u16(json.code).map_err(|err| {
        format!("Invalid response status code {}: {err}", json.code)
    })?);

    for h in json.header {
        rv = rv.header(h.name, h.value);
    }

    rv.body(body)
        .map_err(|err| format!("Error when constructing a response: {err}"))
}

async fn ensure_credit(state: &BridgeState) -> Result<(), String> {
    {
        let mut guard = state.credits.lock().await;
        if *guard > 0 {
            *guard = guard.saturating_sub(1);
            return Ok(());
        }
    }

    let gateway_addr = state
        .gateway_address
        .lock()
        .await
        .clone()
        .ok_or_else(|| "gateway payment address not ready".to_string())?;

    let params = state.payment_params.lock().await.clone();
    let amount = params
        .lovelace_per_request
        .saturating_mul(params.requests_per_microtransaction);

    state
        .hydra
        .send_payment(amount, gateway_addr)
        .await
        .map_err(|e| format!("hydra payment failed: {e}"))?;

    let mut guard = state.credits.lock().await;
    *guard = guard.saturating_add(params.requests_per_microtransaction);
    *guard = guard.saturating_sub(1);
    Ok(())
}

async fn send_ws_msg<J>(
    socket_tx: &mpsc::Sender<tokio_tungstenite::tungstenite::Message>,
    msg: &J,
) -> Result<(), String>
where
    J: ?Sized + serde::ser::Serialize,
{
    let json = serde_json::to_string(msg).map_err(|e| e.to_string())?;
    socket_tx
        .send(tokio_tungstenite::tungstenite::Message::Text(json))
        .await
        .map_err(|_| "broken connection".to_string())
}

fn normalize_gateway_ws_url(input: &str) -> String {
    let mut url = input.trim().to_string();
    if url.starts_with("http://") {
        url = url.replacen("http://", "ws://", 1);
    } else if url.starts_with("https://") {
        url = url.replacen("https://", "wss://", 1);
    } else if !url.starts_with("ws://") && !url.starts_with("wss://") {
        url = format!("wss://{url}");
    }

    if !url.contains("/sdk/ws") {
        if url.ends_with('/') {
            url.push_str("sdk/ws");
        } else {
            url.push_str("/sdk/ws");
        }
    }

    url
}

async fn derive_enterprise_address_from_vkey_json(
    vkey_json: &serde_json::Value,
    network: &Network,
    node_socket_path: &PathBuf,
) -> Result<String> {
    let cardano_cli_exe =
        find_libexec::find_libexec("cardano-cli", "CARDANO_CLI_PATH", &["version"])
            .map_err(|e| anyhow!(e))?;

    let mut cmd = tokio::process::Command::new(&cardano_cli_exe);
    cmd.envs([
        (
            "CARDANO_NODE_SOCKET_PATH",
            node_socket_path.to_string_lossy().to_string(),
        ),
        (
            "CARDANO_NODE_NETWORK_ID",
            match network {
                Network::Mainnet => network.as_str().to_string(),
                other => other.network_magic().to_string(),
            },
        ),
    ]);

    let mut child = cmd
        .args([
            "address",
            "build",
            "--payment-verification-key-file",
            "/dev/stdin",
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .spawn()?;

    {
        let stdin = child.stdin.as_mut().ok_or(anyhow!(
            "failed to open stdin for cardano-cli address build"
        ))?;
        use tokio::io::AsyncWriteExt;
        let bytes = serde_json::to_vec(vkey_json)?;
        stdin.write_all(&bytes).await?;
    }

    let out = child.wait_with_output().await?;
    if !out.status.success() {
        return Err(anyhow!(
            "cardano-cli address build failed: {}",
            String::from_utf8_lossy(&out.stderr)
        ));
    }

    let address = String::from_utf8(out.stdout)?.trim().to_string();
    if address.is_empty() {
        return Err(anyhow!("derived address is empty"));
    }

    Ok(address)
}
