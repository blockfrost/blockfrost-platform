use crate::hydra_server_bridge;
use axum::Extension;
use axum::extract::ws::{Message, WebSocket, WebSocketUpgrade};
use axum::response::IntoResponse;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};
use uuid::Uuid;

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const MAX_BODY_BYTES: usize = 1024 * 1024;
const WS_PING_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Clone)]
pub struct SdkBridgeState {
    router: axum::Router,
    hydras: Option<hydra_server_bridge::HydrasManager>,
}

impl SdkBridgeState {
    pub fn new(router: axum::Router, hydras: Option<hydra_server_bridge::HydrasManager>) -> Self {
        Self { router, hydras }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RequestId(Uuid);

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRequest {
    pub id: RequestId,
    pub method: JsonRequestMethod,
    pub path: String,
    pub header: Vec<JsonHeader>,
    pub body_base64: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonResponse {
    pub id: RequestId,
    pub code: u16,
    pub header: Vec<JsonHeader>,
    pub body_base64: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonHeader {
    pub name: String,
    pub value: String,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Serialize, Deserialize, Debug)]
pub enum JsonRequestMethod {
    GET,
    POST,
}

/// The WebSocket messages that we send.
#[derive(Serialize, Deserialize, Debug)]
pub enum GatewayMessage {
    Response(JsonResponse),
    HydraKExResponse(hydra_server_bridge::KeyExchangeResponse),
    HydraTunnel(hydra_server_bridge::tunnel2::TunnelMsg),
    Ping(u64),
    Pong(u64),
    Error { code: u64, msg: String },
}

/// The WebSocket messages that we receive.
#[derive(Serialize, Deserialize, Debug)]
pub enum BridgeMessage {
    Request(JsonRequest),
    HydraKExRequest(hydra_server_bridge::KeyExchangeRequest),
    HydraTunnel(hydra_server_bridge::tunnel2::TunnelMsg),
    Ping(u64),
    Pong(u64),
}

pub async fn websocket_route(
    ws: WebSocketUpgrade,
    Extension(state): Extension<SdkBridgeState>,
) -> impl IntoResponse {
    ws.on_upgrade(|socket| event_loop::run(state, socket))
}

/// The WebSocket event loop, passing messages between HTTP<->WebSocket, keeping
/// track of persistent connection liveness, etc.
pub mod event_loop {
    use super::*;
    use axum::http::StatusCode;

    /// For clarity, let’s have a single connection 'event_loop per WebSocket
    /// connection, with the following events:
    enum BridgeEvent {
        NewBridgeMessage(BridgeMessage),
        NewResponse(JsonResponse),
        PingTick,
        Finish(String),
    }

    /// Top-level logic of a single WebSocket connection with a bridge.
    pub async fn run(state: SdkBridgeState, socket: WebSocket) {
        let (event_tx, mut event_rx) = mpsc::channel::<BridgeEvent>(64);
        let (socket_tx, response_task, arbitrary_msg_task) =
            wire_socket(event_tx.clone(), socket).await;

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

        // Schedule the first `PingTick` immediately, otherwise we won’t start
        // checking for ping timeout:
        let _ignored_failure: Result<_, _> = event_tx.send(BridgeEvent::PingTick).await;

        // Event loop state (let’s keep it minimal, please):
        let mut last_ping_sent_at: Option<std::time::Instant> = None;
        let mut last_ping_id: u64 = 0;
        let mut disconnection_reason = None;

        let mut initial_hydra_kex: Option<(
            hydra_server_bridge::KeyExchangeRequest,
            hydra_server_bridge::KeyExchangeResponse,
        )> = None;
        let mut hydra_controller: Option<hydra_server_bridge::HydraController> = None;

        let tunnel_cancellation = CancellationToken::new();
        let mut tunnel_controller: Option<hydra_server_bridge::tunnel2::Tunnel> = None;

        // The actual connection event loop:
        'event_loop: while let Some(msg) = event_rx.recv().await {
            match msg {
                BridgeEvent::Finish(reason) => {
                    disconnection_reason = Some(reason);
                    break 'event_loop;
                },

                BridgeEvent::NewBridgeMessage(BridgeMessage::HydraTunnel(tun_msg)) => {
                    if let Some(tunnel_ctl) = &tunnel_controller {
                        match tunnel_ctl.on_msg(tun_msg).await {
                            Ok(()) => (),
                            Err(err) => error!(
                                "hydra-tunnel: got an error when passing message through WebSocket: {err}; ignoring"
                            ),
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
                            match hydras.spawn_new(initial_kex, req).await {
                                Ok((ctl, resp)) => {
                                    hydra_controller = Some(ctl);

                                    // Only start the TCP-over-WebSocket tunnels if we’re running
                                    // on different machines:
                                    if bridge_machine_id != resp.machine_id {
                                        let (tunnel_ctl, mut tunnel_rx) =
                                            hydra_server_bridge::tunnel2::Tunnel::new(
                                                hydra_server_bridge::tunnel2::TunnelConfig {
                                                    expose_port: resp.gateway_h2h_port,
                                                    id_prefix_bit: true,
                                                    ..(hydra_server_bridge::tunnel2::TunnelConfig::default())
                                                },
                                                tunnel_cancellation.clone(),
                                            );

                                        tunnel_ctl.spawn_listener(resp.proposed_platform_h2h_port).await.expect("FIXME: this really shouldn’t fail, unless we hit the TOCTOU race condition…");

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

                                    GatewayMessage::HydraKExResponse(resp)
                                },
                                Err(err) => GatewayMessage::Error {
                                    code: 537,
                                    msg: format!("Hydra micropayments setup error: {err}"),
                                },
                            }
                        },
                        (false, Some(hydras), _, _) => {
                            match hydras.initialize_key_exchange(req.clone()).await {
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
                    let request_id = request.id.clone();

                    let response = match &hydra_controller {
                        None => Some(error_response(
                            request_id,
                            StatusCode::SERVICE_UNAVAILABLE,
                            "Hydra head is not ready".to_string(),
                        )),
                        Some(ctl) => {
                            if !ctl.is_alive() {
                                Some(error_response(
                                    request_id,
                                    StatusCode::SERVICE_UNAVAILABLE,
                                    "Hydra controller is not running".to_string(),
                                ))
                            } else {
                                match ctl.try_consume_credit() {
                                    Ok(()) => None,
                                    Err(hydra_server_bridge::CreditError::HeadNotOpen) => {
                                        Some(error_response(
                                            request_id,
                                            StatusCode::SERVICE_UNAVAILABLE,
                                            "Hydra head is not open".to_string(),
                                        ))
                                    },
                                    Err(hydra_server_bridge::CreditError::InsufficientCredits) => {
                                        Some(error_response(
                                            request_id,
                                            StatusCode::PAYMENT_REQUIRED,
                                            "Prepaid credits exhausted".to_string(),
                                        ))
                                    },
                                }
                            }
                        },
                    };

                    if let Some(response) = response {
                        if send_json_msg(&socket_tx, &GatewayMessage::Response(response))
                            .await
                            .is_err()
                        {
                            break 'event_loop;
                        }
                    } else {
                        let router = state.router.clone();
                        let event_tx = event_tx.clone();
                        tokio::spawn(async move {
                            let response = handle_one(router, request).await;
                            let _ignored_failure: Result<_, _> =
                                event_tx.send(BridgeEvent::NewResponse(response)).await;
                        });
                    }
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
                        // Ping timeout:
                        disconnection_reason = Some("ping timeout".to_string());
                        break 'event_loop;
                    } else {
                        // The periodic `PingTick` loop:
                        schedule_ping_tick();
                        // Time to send a new ping:
                        last_ping_id += 1;
                        last_ping_sent_at = Some(std::time::Instant::now());
                        if send_json_msg(&socket_tx, &GatewayMessage::Ping(last_ping_id))
                            .await
                            .is_err()
                        {
                            break 'event_loop;
                        }
                    }
                },
            }
        }

        if let Some(ctl) = hydra_controller {
            ctl.terminate().await
        }

        tunnel_cancellation.cancel();

        let disconnection_reason_ = disconnection_reason
            .clone()
            .unwrap_or("reason unknown".to_string());

        warn!(
            "sdk-bridge-ws: connection event loop finished: {}",
            disconnection_reason_
        );

        let _ignored_failure: Result<_, _> = socket_tx
            .send(Message::Close(disconnection_reason.map(|why| {
                axum::extract::ws::CloseFrame {
                    code: tungstenite::protocol::frame::coding::CloseCode::Normal.into(),
                    reason: why.into(),
                }
            })))
            .await;

        // Wait for all children to finish:
        let children = [response_task, arbitrary_msg_task];
        children.iter().for_each(|t| t.abort());
        futures::future::join_all(children).await;

        info!("sdk-bridge-ws: connection closed");
    }

    async fn wire_socket(
        event_tx: mpsc::Sender<BridgeEvent>,
        socket: WebSocket,
    ) -> (mpsc::Sender<Message>, JoinHandle<()>, JoinHandle<()>) {
        use futures_util::{SinkExt, StreamExt};

        let (msg_tx, mut msg_rx) = mpsc::channel::<Message>(64);
        let (mut sock_tx, mut sock_rx) = socket.split();
        let response_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = sock_rx.next().await {
                match msg {
                    Message::Text(text) => {
                        match serde_json::from_str::<BridgeMessage>(&text) {
                            Ok(msg) => {
                                if event_tx
                                    .send(BridgeEvent::NewBridgeMessage(msg))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            },
                            Err(err) => warn!(
                                "sdk-bridge-ws: received unparsable text message: {:?}: {:?}",
                                text, err,
                            ),
                        };
                    },
                    Message::Binary(bin) => {
                        warn!(
                            "sdk-bridge-ws: received unexpected binary message: {:?}",
                            hex::encode(bin),
                        );
                    },
                    Message::Close(frame) => {
                        warn!(
                            "sdk-bridge-ws: bridge disconnected (CloseFrame: {:?})",
                            frame,
                        );
                        let _ignored_failure: Result<_, _> = event_tx
                            .send(BridgeEvent::Finish("bridge disconnected".to_string()))
                            .await;
                        break;
                    },
                    Message::Ping(_) | Message::Pong(_) => {},
                }
            }
        });
        let arbitrary_msg_task = tokio::spawn(async move {
            while let Some(msg) = msg_rx.recv().await {
                match sock_tx.send(msg).await {
                    Ok(()) => (),
                    Err(err) => {
                        error!("sdk-bridge-ws: error when sending a message: {:?}", err);
                        break;
                    },
                }
            }
        });
        (msg_tx, response_task, arbitrary_msg_task)
    }

    async fn send_json_msg<J>(socket_tx: &mpsc::Sender<Message>, msg: &J) -> Result<(), String>
    where
        J: ?Sized + serde::ser::Serialize,
    {
        match serde_json::to_string(msg) {
            Ok(msg) => match socket_tx.send(Message::Text(msg)).await {
                Ok(_) => Ok(()),
                Err(err) => {
                    error!("sdk-bridge-ws: error when sending a message: {:?}", err);
                    Err("broken connection with the bridge".to_string())
                },
            },
            Err(err) => {
                let err = format!(
                    "error when serializing request to JSON (this will never happen): {err:?}"
                );
                error!("sdk-bridge-ws: {}", err);
                Err(err)
            },
        }
    }

    fn error_response(request_id: RequestId, code: StatusCode, why: String) -> JsonResponse {
        JsonResponse {
            id: request_id,
            code: code.as_u16(),
            header: vec![],
            body_base64: why,
        }
    }
}

/// Passes one [`JsonRequest`] through our underlying original HTTP server.
/// Everything happens internally, in memory, without opening new TCP
/// connections etc. – very light.
async fn handle_one(http_router: axum::Router, request: JsonRequest) -> JsonResponse {
    use axum::body::Body;
    use hyper::StatusCode;
    use hyper::{Request, Response};
    use tower::ServiceExt;

    let request_id = request.id.clone();
    let request_id_ = request.id.clone();

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
        Err((code, err)) => {
            error!("sdk-bridge-ws: returning {}, because: {}", code, err);
            JsonResponse {
                id: request_id_,
                code: code.into(),
                header: vec![],
                body_base64: err,
            }
        },
    }
}

fn json_to_request(
    json: JsonRequest,
) -> Result<hyper::Request<axum::body::Body>, (hyper::StatusCode, String)> {
    use axum::body::Body;
    use axum::http::{Method, StatusCode};
    use hyper::Request;

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

    let method = match json.method {
        JsonRequestMethod::GET => Method::GET,
        JsonRequestMethod::POST => Method::POST,
    };

    let mut rv = Request::builder().method(method).uri(json.path);

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
    request_id: RequestId,
) -> Result<JsonResponse, (hyper::StatusCode, String)> {
    use hyper::StatusCode;

    let header: Vec<JsonHeader> = response
        .headers()
        .iter()
        .flat_map(|(name, value)| {
            value.to_str().ok().map(|value| JsonHeader {
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
