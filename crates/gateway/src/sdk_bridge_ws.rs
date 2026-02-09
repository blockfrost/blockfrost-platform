use crate::hydra;
use crate::load_balancer::{JsonRequest, JsonResponse};
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::{Extension, response::IntoResponse};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, warn};

const WS_PING_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
const MAX_BODY_BYTES: usize = 1024 * 1024;

#[derive(Clone)]
pub struct SdkBridgeState {
    pub http_router: axum::Router,
    pub hydras: Option<hydra::server::HydrasManager>,
}

pub async fn websocket_route(
    ws: WebSocketUpgrade,
    Extension(state): Extension<SdkBridgeState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| event_loop::run(state, socket))
}

#[derive(Serialize, Deserialize, Debug)]
enum BridgeMessage {
    Request(JsonRequest),
    HydraKExRequest(hydra::server::KeyExchangeRequest),
    HydraTunnel(hydra::tunnel2::TunnelMsg),
    Ping(u64),
    Pong(u64),
}

#[derive(Serialize, Deserialize, Debug)]
enum GatewayMessage {
    Response(JsonResponse),
    HydraKExResponse(hydra::server::KeyExchangeResponse),
    HydraTunnel(hydra::tunnel2::TunnelMsg),
    Ping(u64),
    Pong(u64),
    Error { code: u64, msg: String },
}

#[derive(Debug, Default)]
struct Credits {
    available: u64,
    last_balance: u64,
}

mod event_loop {
    use super::*;

    enum BridgeEvent {
        NewBridgeMessage(BridgeMessage),
        NewResponse(JsonResponse),
        PingTick,
        SocketError(String),
    }

    pub async fn run(state: SdkBridgeState, socket: WebSocket) {
        let (event_tx, mut event_rx) = mpsc::channel::<BridgeEvent>(64);
        let (socket_tx, request_task, socket_task) = wire_socket(event_tx.clone(), socket).await;

        let schedule_ping_tick = {
            let event_tx = event_tx.clone();
            move || {
                let tx = event_tx.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(WS_PING_TIMEOUT).await;
                    let _ignored_failure: Result<_, _> = tx.send(BridgeEvent::PingTick).await;
                })
            }
        };

        schedule_ping_tick();

        let tunnel_cancellation = CancellationToken::new();
        let mut tunnel_controller: Option<hydra::tunnel2::Tunnel> = None;
        let credits = Arc::new(Mutex::new(Credits::default()));
        let mut credit_task: Option<JoinHandle<()>> = None;

        let mut last_ping_sent_at: Option<std::time::Instant> = None;
        let mut last_ping_id: u64 = 0;

        let mut initial_hydra_kex: Option<(
            hydra::server::KeyExchangeRequest,
            hydra::server::KeyExchangeResponse,
        )> = None;
        let mut hydra_controller: Option<hydra::server::HydraController> = None;

        'event_loop: while let Some(msg) = event_rx.recv().await {
            match msg {
                BridgeEvent::SocketError(err) => {
                    warn!("sdk-bridge: socket error: {err}");
                    break 'event_loop;
                },

                BridgeEvent::NewBridgeMessage(BridgeMessage::HydraTunnel(tun_msg)) => {
                    if let Some(tunnel_ctl) = &tunnel_controller {
                        if let Err(err) = tunnel_ctl.on_msg(tun_msg).await {
                            error!(
                                "sdk-bridge: hydra-tunnel: error when passing message: {err}; ignoring"
                            );
                        }
                    }
                },

                BridgeEvent::NewBridgeMessage(BridgeMessage::HydraKExRequest(req)) => {
                    let already_exists = match &hydra_controller {
                        None => false,
                        Some(ctl) => ctl.is_alive(),
                    };

                    let reply = match (
                        already_exists,
                        &state.hydras,
                        &req.accepted_platform_h2h_port,
                        initial_hydra_kex.take(),
                    ) {
                        (true, _, _, _) => GatewayMessage::Error {
                            code: 538,
                            msg: "Hydra controller already exists on this connection".to_string(),
                        },
                        (false, None, _, _) => GatewayMessage::Error {
                            code: 536,
                            msg: "Hydra micropayments not supported".to_string(),
                        },
                        (false, Some(hydras), Some(_accepted_port), Some(initial_kex)) => {
                            let bridge_machine_id = req.machine_id.clone();
                            match hydras.spawn_new(&crate::types::AssetName("sdk-bridge".into()), "", initial_kex, req).await {
                                Ok((ctl, resp)) => {
                                    hydra_controller = Some(ctl.clone());

                                    if bridge_machine_id != resp.machine_id {
                                        let (tunnel_ctl, mut tunnel_rx) =
                                            hydra::tunnel2::Tunnel::new(
                                                hydra::tunnel2::TunnelConfig {
                                                    expose_port: resp.gateway_h2h_port,
                                                    id_prefix_bit: true,
                                                    ..(hydra::tunnel2::TunnelConfig::default())
                                                },
                                                tunnel_cancellation.clone(),
                                            );

                                        tunnel_ctl.spawn_listener(resp.proposed_platform_h2h_port).await.expect("tunnel listener should start");

                                        let socket_tx_ = socket_tx.clone();
                                        tokio::spawn(async move {
                                            while let Some(tun_msg) = tunnel_rx.recv().await {
                                                if send_json_msg(
                                                    &socket_tx_,
                                                    &GatewayMessage::HydraTunnel(tun_msg),
                                                )
                                                .await
                                                .is_err()
                                                {
                                                    break;
                                                }
                                            }
                                        });

                                        tunnel_controller = Some(tunnel_ctl);
                                    }

                                    if credit_task.is_none() {
                                        let credits = credits.clone();
                                        let params = hydras.payment_params();
                                        let address = hydras.gateway_payment_address();
                                        let cancel = tunnel_cancellation.clone();
                                        credit_task = Some(tokio::spawn(async move {
                                            let api_port = ctl.wait_api_port().await;
                                            if api_port == 0 {
                                                return;
                                            }
                                            watch_credits(
                                                credits,
                                                params,
                                                address,
                                                api_port,
                                                cancel,
                                            )
                                            .await;
                                        }));
                                    }

                                    GatewayMessage::HydraKExResponse(resp)
                                },
                                Err(err) => GatewayMessage::Error {
                                    code: 537,
                                    msg: format!("Hydra micropayments setup error: {err}"),
                                },
                            }
                        },
                        (false, Some(hydras), _, _) => {
                            match hydras
                                .initialize_key_exchange(&crate::types::AssetName("sdk-bridge".into()), req.clone())
                                .await
                            {
                                Ok(resp) => {
                                    initial_hydra_kex = Some((req, resp.clone()));
                                    GatewayMessage::HydraKExResponse(resp)
                                },
                                Err(err) => GatewayMessage::Error {
                                    code: 537,
                                    msg: format!("Hydra micropayments setup error: {err}"),
                                },
                            }
                        },
                    };

                    if send_json_msg(&socket_tx, &reply).await.is_err() {
                        break 'event_loop;
                    }
                },

                BridgeEvent::NewBridgeMessage(BridgeMessage::Request(request)) => {
                    if state.hydras.is_none() {
                        let response = error_response(
                            request.id.clone(),
                            503,
                            "Hydra micropayments are not configured on this gateway",
                        );
                        let _ = event_tx.send(BridgeEvent::NewResponse(response)).await;
                        continue;
                    }

                    let mut credits_guard = credits.lock().await;
                    if credits_guard.available == 0 {
                        let response = error_response(
                            request.id.clone(),
                            402,
                            "Hydra credits depleted; please prepay", 
                        );
                        let _ = event_tx.send(BridgeEvent::NewResponse(response)).await;
                        continue;
                    }
                    credits_guard.available -= 1;
                    drop(credits_guard);

                    let router = state.http_router.clone();
                    let event_tx = event_tx.clone();
                    tokio::spawn(async move {
                        let response = handle_one(router, request).await;
                        let _ignored_failure: Result<_, _> =
                            event_tx.send(BridgeEvent::NewResponse(response)).await;
                    });
                },

                BridgeEvent::NewResponse(response) => {
                    if send_json_msg(&socket_tx, &GatewayMessage::Response(response))
                        .await
                        .is_err()
                    {
                        break 'event_loop;
                    }
                },

                BridgeEvent::NewBridgeMessage(BridgeMessage::Ping(ping_id)) => {
                    if send_json_msg(&socket_tx, &GatewayMessage::Pong(ping_id))
                        .await
                        .is_err()
                    {
                        break 'event_loop;
                    }
                },

                BridgeEvent::NewBridgeMessage(BridgeMessage::Pong(pong_id)) => {
                    if pong_id == last_ping_id {
                        last_ping_sent_at = None;
                    }
                },

                BridgeEvent::PingTick => {
                    if let Some(_sent_at) = last_ping_sent_at {
                        break 'event_loop;
                    }

                    schedule_ping_tick();
                    last_ping_id += 1;
                    last_ping_sent_at = Some(std::time::Instant::now());
                    if send_json_msg(&socket_tx, &GatewayMessage::Ping(last_ping_id))
                        .await
                        .is_err()
                    {
                        break 'event_loop;
                    }
                },
            }
        }

        tunnel_cancellation.cancel();

        let children = [request_task, socket_task];
        children.iter().for_each(|t| t.abort());
        futures::future::join_all(children).await;
    }

    async fn wire_socket(
        event_tx: mpsc::Sender<BridgeEvent>,
        socket: WebSocket,
    ) -> (mpsc::Sender<Message>, JoinHandle<()>, JoinHandle<()>) {
        use futures_util::{SinkExt, StreamExt};

        let (msg_tx, mut msg_rx) = mpsc::channel::<Message>(64);
        let (mut sock_tx, mut sock_rx) = socket.split();

        let request_task = tokio::spawn(async move {
            'read_loop: loop {
                match sock_rx.next().await {
                    None => {
                        let _ignored_failure: Result<_, _> = event_tx
                            .send(BridgeEvent::SocketError("connection closed".to_string()))
                            .await;
                        break 'read_loop;
                    },
                    Some(Err(err)) => {
                        let _ignored_failure: Result<_, _> = event_tx
                            .send(BridgeEvent::SocketError(format!("stream error: {err:?}")))
                            .await;
                        break 'read_loop;
                    },
                    Some(Ok(Message::Close(frame))) => {
                        warn!("sdk-bridge: relay disconnected (CloseFrame: {:?})", frame);
                        let _ignored_failure: Result<_, _> = event_tx
                            .send(BridgeEvent::SocketError("relay disconnected".to_string()))
                            .await;
                        break 'read_loop;
                    },
                    Some(Ok(Message::Binary(bin))) => {
                        warn!(
                            "sdk-bridge: received unexpected binary message: {:?}",
                            hex::encode(bin),
                        );
                    },
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<BridgeMessage>(&text) {
                            Ok(msg) => {
                                if event_tx
                                    .send(BridgeEvent::NewBridgeMessage(msg))
                                    .await
                                    .is_err()
                                {
                                    break 'read_loop;
                                }
                            },
                            Err(err) => warn!(
                                "sdk-bridge: received unparsable text message: {:?}: {:?}",
                                text, err,
                            ),
                        };
                    },
                    Some(Ok(Message::Ping(_) | Message::Pong(_))) => {},
                }
            }
        });

        let socket_task = tokio::spawn(async move {
            while let Some(msg) = msg_rx.recv().await {
                if let Err(err) = sock_tx.send(msg).await {
                    error!("sdk-bridge: error when sending a message: {:?}", err);
                    break;
                }
            }
        });

        (msg_tx, request_task, socket_task)
    }

    async fn send_json_msg<J>(socket_tx: &mpsc::Sender<Message>, msg: &J) -> Result<(), String>
    where
        J: ?Sized + serde::ser::Serialize,
    {
        match serde_json::to_string(msg) {
            Ok(msg) => socket_tx
                .send(Message::Text(msg))
                .await
                .map_err(|_| "broken connection".to_string()),
            Err(err) => Err(format!("error when serializing request to JSON: {err:?}")),
        }
    }

    async fn handle_one(http_router: axum::Router, request: JsonRequest) -> JsonResponse {
        use axum::body::Body;
        use hyper::StatusCode;
        use hyper::{Request, Response};
        use tower::ServiceExt;

        let request_id = request.id.clone();
        let request_id_for_error = request.id.clone();

        let rv: Result<JsonResponse, (StatusCode, String)> = async {
            let req: Request<Body> = json_to_request(request)?;

            let response: Response<Body> =
                tokio::time::timeout(REQUEST_TIMEOUT, http_router.into_service().oneshot(req))
                    .await
                    .map_err(|_elapsed| {
                        (
                            StatusCode::GATEWAY_TIMEOUT,
                            format!("Timed out while waiting {REQUEST_TIMEOUT:?} for a response"),
                        )
                    })?
                    .unwrap();

            response_to_json(response, request_id).await
        }
        .await;

        match rv {
            Ok(ok) => ok,
            Err((code, err)) => error_response(request_id_for_error, code.as_u16(), &err),
        }
    }

    fn json_to_request(
        json: JsonRequest,
    ) -> Result<hyper::Request<axum::body::Body>, (hyper::StatusCode, String)> {
        use axum::body::Body;
        use hyper::Request;
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
                            (
                                StatusCode::BAD_REQUEST,
                                format!("Invalid base64 encoding of body_base64: {err}"),
                            )
                        })?;
                Body::from(body_bytes)
            }
        };

        let mut rv = Request::builder()
            .method(json.method.as_str())
            .uri(json.path);

        for h in json.header {
            rv = rv.header(h.name, h.value);
        }

        rv.body(body).map_err(|err| {
            (
                StatusCode::BAD_REQUEST,
                format!("Error when constructing a request from JSON request: {err}"),
            )
        })
    }

    async fn response_to_json(
        response: hyper::Response<axum::body::Body>,
        request_id: crate::load_balancer::RequestId,
    ) -> Result<JsonResponse, (hyper::StatusCode, String)> {
        use hyper::StatusCode;

        let header = response
            .headers()
            .iter()
            .flat_map(|(name, value)| {
                value.to_str().ok().map(|value| crate::load_balancer::JsonHeader {
                    name: name.to_string(),
                    value: value.to_string(),
                })
            })
            .collect();

        let code: u16 = response.status().into();

        let body_base64: String = {
            let body = response.into_body();
            let body_bytes = axum::body::to_bytes(body, MAX_BODY_BYTES)
                .await
                .map_err(|err| {
                    (
                        StatusCode::BAD_GATEWAY,
                        format!("Cannot read body of the response: {err}"),
                    )
                })?;
            use base64::{Engine as _, engine::general_purpose};
            general_purpose::STANDARD.encode(body_bytes)
        };

        Ok(JsonResponse {
            id: request_id,
            code,
            header,
            body_base64,
        })
    }

    fn error_response(request_id: crate::load_balancer::RequestId, code: u16, msg: &str) -> JsonResponse {
        use base64::{Engine as _, engine::general_purpose};
        JsonResponse {
            id: request_id,
            code,
            header: vec![],
            body_base64: general_purpose::STANDARD.encode(msg.as_bytes()),
        }
    }

    async fn watch_credits(
        credits: Arc<Mutex<Credits>>,
        params: hydra::server::HydraPaymentParams,
        address: String,
        api_port: u16,
        cancel: CancellationToken,
    ) {
        let microtx_lovelace = params
            .lovelace_per_request
            .saturating_mul(params.requests_per_microtransaction);
        if microtx_lovelace == 0 {
            return;
        }

        loop {
            if cancel.is_cancelled() {
                break;
            }

            match snapshot_lovelace_for_address(api_port, &address).await {
                Ok(current) => {
                    let mut guard = credits.lock().await;
                    if current < guard.last_balance {
                        guard.available = 0;
                        guard.last_balance = current;
                    } else if current > guard.last_balance {
                        let delta = current - guard.last_balance;
                        let gained = delta / microtx_lovelace;
                        if gained > 0 {
                            guard.available = guard
                                .available
                                .saturating_add(gained.saturating_mul(params.requests_per_microtransaction));
                            guard.last_balance = guard
                                .last_balance
                                .saturating_add(gained.saturating_mul(microtx_lovelace));
                        }
                    }
                },
                Err(err) => warn!("sdk-bridge: failed to fetch hydra snapshot: {err}"),
            }

            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
        }
    }

    async fn snapshot_lovelace_for_address(
        api_port: u16,
        address: &str,
    ) -> Result<u64, String> {
        use anyhow::Context;

        let url = format!("http://127.0.0.1:{}/snapshot/utxo", api_port);

        let v: serde_json::Value = reqwest::Client::new()
            .get(&url)
            .send()
            .await
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .await
            .map_err(|e| e.to_string())?;

        let obj = v
            .as_object()
            .context("snapshot/utxo: expected top-level JSON object")
            .map_err(|e| e.to_string())?;

        let mut total: u64 = 0;
        for (_k, entry) in obj.iter() {
            if entry.get("address").and_then(serde_json::Value::as_str) != Some(address) {
                continue;
            }
            if let Some(value_obj) = entry.get("value").and_then(|v| v.as_object()) {
                if let Some(lovelace_val) = value_obj.get("lovelace") {
                    total = total
                        .checked_add(as_u64(lovelace_val).map_err(|e| e.to_string())?)
                        .ok_or_else(|| "cannot add".to_string())?;
                    continue;
                }
            }
            if let Some(amount_arr) = entry.get("amount").and_then(|v| v.as_array()) {
                if let Some(lovelace_val) = amount_arr.first() {
                    total = total
                        .checked_add(as_u64(lovelace_val).map_err(|e| e.to_string())?)
                        .ok_or_else(|| "cannot add".to_string())?;
                }
            }
        }

        Ok(total)
    }

    fn as_u64(v: &serde_json::Value) -> Result<u64, anyhow::Error> {
        if let Some(n) = v.as_u64() {
            return Ok(n);
        }
        if let Some(s) = v.as_str() {
            return Ok(s.parse()?);
        }
        Err(anyhow::anyhow!("lovelace value is neither u64 nor string"))
    }
}
