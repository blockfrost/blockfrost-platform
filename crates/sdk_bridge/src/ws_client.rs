use crate::hydra_client;
use crate::protocol::{JsonRequest, JsonResponse, RequestId};
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::time::Duration;
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tracing::{error, info, warn};

const REQUEST_TIMEOUT: Duration = Duration::from_secs(60);
const WS_PING_TIMEOUT: Duration = Duration::from_secs(13);

#[derive(Clone)]
pub struct BridgeHandle {
    request_tx: mpsc::Sender<BridgeRequest>,
    hydra: hydra_client::HydraController,
}

impl BridgeHandle {
    pub async fn forward_request(&self, request: JsonRequest) -> Result<JsonResponse, BridgeError> {
        let (tx, rx) = oneshot::channel();
        self.request_tx
            .send(BridgeRequest {
                request,
                respond_to: tx,
            })
            .await
            .map_err(|_| BridgeError::ConnectionClosed)?;

        match tokio::time::timeout(REQUEST_TIMEOUT, rx).await {
            Ok(Ok(response)) => Ok(response),
            Ok(Err(_)) => Err(BridgeError::ResponseDropped),
            Err(_) => Err(BridgeError::Timeout),
        }
    }

    pub fn hydra(&self) -> &hydra_client::HydraController {
        &self.hydra
    }
}

#[derive(Debug)]
pub enum BridgeError {
    ConnectionClosed,
    Timeout,
    ResponseDropped,
}

pub struct BridgeWsConfig {
    pub ws_url: String,
    pub hydra: hydra_client::HydraConfig,
}

pub async fn start(config: BridgeWsConfig) -> Result<BridgeHandle> {
    let (request_tx, request_rx) = mpsc::channel(64);
    let (kex_request_tx, kex_request_rx) = mpsc::channel(32);
    let (kex_response_tx, kex_response_rx) = mpsc::channel(32);
    let (terminate_tx, terminate_rx) = mpsc::channel(1);

    let hydra = hydra_client::HydraController::spawn(
        config.hydra,
        kex_request_tx,
        kex_response_rx,
        terminate_rx,
    )
    .await?;

    tokio::spawn(run_ws_loop(
        config.ws_url,
        request_rx,
        kex_request_rx,
        kex_response_tx,
        terminate_tx,
    ));

    Ok(BridgeHandle { request_tx, hydra })
}

struct BridgeRequest {
    request: JsonRequest,
    respond_to: oneshot::Sender<JsonResponse>,
}

/// The WebSocket messages that we receive.
#[derive(Serialize, Deserialize, Debug)]
enum GatewayMessage {
    Response(JsonResponse),
    HydraKExResponse(hydra_client::KeyExchangeResponse),
    HydraTunnel(hydra_client::tunnel2::TunnelMsg),
    Ping(u64),
    Pong(u64),
    Error { code: u64, msg: String },
}

/// The WebSocket messages that we send.
#[derive(Serialize, Deserialize, Debug)]
enum BridgeMessage {
    Request(JsonRequest),
    HydraKExRequest(hydra_client::KeyExchangeRequest),
    HydraTunnel(hydra_client::tunnel2::TunnelMsg),
    Ping(u64),
    Pong(u64),
}

async fn run_ws_loop(
    ws_url: String,
    mut request_rx: mpsc::Receiver<BridgeRequest>,
    mut kex_request_rx: mpsc::Receiver<hydra_client::KeyExchangeRequest>,
    kex_response_tx: mpsc::Sender<hydra_client::KeyExchangeResponse>,
    terminate_tx: mpsc::Sender<hydra_client::TerminateRequest>,
) {
    let connect_result = tokio_tungstenite::connect_async(&ws_url).await;
    let (ws_stream, _response) = match connect_result {
        Ok(ok) => ok,
        Err(err) => {
            error!("sdk-bridge: failed to connect to {}: {}", ws_url, err);
            let _ = terminate_tx.send(hydra_client::TerminateRequest).await;
            return;
        },
    };

    info!("sdk-bridge: connected to {}", ws_url);

    let (event_tx, mut event_rx) = mpsc::channel::<BridgeEvent>(64);
    let (socket_tx, request_task, arbitrary_msg_task) =
        wire_socket(event_tx.clone(), ws_stream, ws_url.clone()).await;

    let inflight: std::sync::Arc<Mutex<HashMap<RequestId, oneshot::Sender<JsonResponse>>>> =
        std::sync::Arc::new(Mutex::new(HashMap::new()));

    let kex_fwd_task = {
        let event_tx = event_tx.clone();
        tokio::spawn(async move {
            while let Some(req) = kex_request_rx.recv().await {
                let _ = event_tx.send(BridgeEvent::HydraKExRequest(req)).await;
            }
        })
    };

    let request_fwd_task = {
        let event_tx = event_tx.clone();
        tokio::spawn(async move {
            while let Some(req) = request_rx.recv().await {
                if event_tx.send(BridgeEvent::NewRequest(req)).await.is_err() {
                    break;
                }
            }
        })
    };

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

    let mut last_ping_sent_at: Option<std::time::Instant> = None;
    let mut last_ping_id: u64 = 0;
    let mut loop_error: Result<(), String> = Ok(());

    // Schedule the first `PingTick` immediately, otherwise we won’t start
    // checking for ping timeout:
    schedule_ping_tick();

    let tunnel_cancellation = CancellationToken::new();
    let mut tunnel_controller: Option<hydra_client::tunnel2::Tunnel> = None;

    'event_loop: while let Some(msg) = event_rx.recv().await {
        match msg {
            BridgeEvent::SocketError(err) => {
                loop_error = Err(err);
                break 'event_loop;
            },

            BridgeEvent::NewGatewayMessage(GatewayMessage::HydraTunnel(tun_msg)) => {
                if let Some(tunnel_ctl) = &tunnel_controller {
                    match tunnel_ctl.on_msg(tun_msg).await {
                        Ok(()) => (),
                        Err(err) => error!(
                            "hydra-tunnel: got an error when passing message through WebSocket: {err}; ignoring"
                        ),
                    }
                }
            },

            BridgeEvent::NewGatewayMessage(GatewayMessage::HydraKExResponse(resp)) => {
                if resp.machine_id != hydra_client::verifications::hashed_machine_id() {
                    let (tunnel_ctl, mut tunnel_rx) = hydra_client::tunnel2::Tunnel::new(
                        hydra_client::tunnel2::TunnelConfig {
                            expose_port: resp.proposed_platform_h2h_port,
                            id_prefix_bit: true,
                            ..(hydra_client::tunnel2::TunnelConfig::default())
                        },
                        tunnel_cancellation.clone(),
                    );

                    if let Err(err) = tunnel_ctl.spawn_listener(resp.gateway_h2h_port).await {
                        warn!("hydra-tunnel: failed to spawn listener: {err}");
                    } else {
                        let socket_tx_ = socket_tx.clone();
                        tokio::spawn(async move {
                            while let Some(tun_msg) = tunnel_rx.recv().await {
                                if send_json_msg(&socket_tx_, &BridgeMessage::HydraTunnel(tun_msg))
                                    .await
                                    .is_err()
                                {
                                    break;
                                }
                            }
                        });

                        tunnel_controller = Some(tunnel_ctl);
                    }
                }

                let _ = kex_response_tx.send(resp).await;
            },

            BridgeEvent::NewGatewayMessage(GatewayMessage::Response(response)) => {
                let request_id = response.id.clone();
                let sender = inflight.lock().await.remove(&request_id);
                if let Some(sender) = sender {
                    let _ = sender.send(response);
                } else {
                    warn!(
                        "sdk-bridge: received response for unknown request: {:?}",
                        request_id
                    );
                }
            },

            BridgeEvent::NewGatewayMessage(GatewayMessage::Error { code, msg }) => {
                warn!("sdk-bridge: gateway error {}: {}", code, msg);
            },

            BridgeEvent::NewGatewayMessage(GatewayMessage::Ping(ping_id)) => {
                if let Err(err) = send_json_msg(&socket_tx, &BridgeMessage::Pong(ping_id)).await {
                    loop_error = Err(err);
                    break 'event_loop;
                }
            },

            BridgeEvent::NewGatewayMessage(GatewayMessage::Pong(pong_id)) => {
                if pong_id == last_ping_id {
                    last_ping_sent_at = None;
                }
            },

            BridgeEvent::PingTick => {
                if let Some(_sent_at) = last_ping_sent_at {
                    loop_error = Err("ping timeout".to_string());
                    break 'event_loop;
                } else {
                    schedule_ping_tick();
                    last_ping_id += 1;
                    last_ping_sent_at = Some(std::time::Instant::now());
                    if let Err(err) =
                        send_json_msg(&socket_tx, &BridgeMessage::Ping(last_ping_id)).await
                    {
                        loop_error = Err(err);
                        break 'event_loop;
                    }
                }
            },

            BridgeEvent::HydraKExRequest(req) => {
                if send_json_msg(&socket_tx, &BridgeMessage::HydraKExRequest(req))
                    .await
                    .is_err()
                {
                    break 'event_loop;
                }
            },

            BridgeEvent::NewRequest(req) => {
                let request_id = req.request.id.clone();
                inflight
                    .lock()
                    .await
                    .insert(request_id.clone(), req.respond_to);
                if let Err(err) =
                    send_json_msg(&socket_tx, &BridgeMessage::Request(req.request)).await
                {
                    loop_error = Err(err);
                    break 'event_loop;
                }
            },
        }
    }

    if let Err(err) = loop_error {
        warn!("sdk-bridge: WebSocket loop finished with error: {err}");
    }

    if let Some(tunnel_ctl) = &tunnel_controller {
        tunnel_ctl.cancel();
    }
    tunnel_cancellation.cancel();

    let _ = terminate_tx.send(hydra_client::TerminateRequest).await;

    let inflight_keys: Vec<RequestId> = inflight.lock().await.keys().cloned().collect();
    for request_id in inflight_keys {
        if let Some(sender) = inflight.lock().await.remove(&request_id) {
            let _ = sender.send(error_response(
                request_id,
                503,
                "gateway WebSocket disconnected".to_string(),
            ));
        }
    }

    let children = [
        request_task,
        arbitrary_msg_task,
        kex_fwd_task,
        request_fwd_task,
    ];
    children.iter().for_each(|t| t.abort());
    futures::future::join_all(children).await;
}

enum BridgeEvent {
    NewGatewayMessage(GatewayMessage),
    NewRequest(BridgeRequest),
    HydraKExRequest(hydra_client::KeyExchangeRequest),
    PingTick,
    SocketError(String),
}

async fn wire_socket(
    event_tx: mpsc::Sender<BridgeEvent>,
    socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    ws_url: String,
) -> (
    mpsc::Sender<tungstenite::protocol::Message>,
    JoinHandle<()>,
    JoinHandle<()>,
) {
    use futures_util::{SinkExt, StreamExt};

    let (msg_tx, mut msg_rx) = mpsc::channel::<tungstenite::protocol::Message>(64);
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
                Some(Ok(tungstenite::protocol::Message::Close(frame))) => {
                    warn!("sdk-bridge: gateway disconnected (CloseFrame: {:?})", frame);
                    let _ignored_failure: Result<_, _> = event_tx
                        .send(BridgeEvent::SocketError("gateway disconnected".to_string()))
                        .await;
                    break 'read_loop;
                },
                Some(Ok(
                    tungstenite::protocol::Message::Frame(_)
                    | tungstenite::protocol::Message::Ping(_)
                    | tungstenite::protocol::Message::Pong(_),
                )) => {},
                Some(Ok(tungstenite::protocol::Message::Binary(bin))) => {
                    warn!(
                        "sdk-bridge: received unexpected binary message: {:?}",
                        hex::encode(bin),
                    );
                },
                Some(Ok(tungstenite::protocol::Message::Text(text))) => {
                    match serde_json::from_str::<GatewayMessage>(&text) {
                        Ok(msg) => {
                            if event_tx
                                .send(BridgeEvent::NewGatewayMessage(msg))
                                .await
                                .is_err()
                            {
                                break 'read_loop;
                            }
                        },
                        Err(err) => warn!(
                            "sdk-bridge: received unparsable text message from {}: {:?}: {:?}",
                            ws_url, text, err,
                        ),
                    };
                },
            }
        }
    });

    let arbitrary_msg_task = tokio::spawn(async move {
        while let Some(msg) = msg_rx.recv().await {
            match sock_tx.send(msg).await {
                Ok(()) => (),
                Err(err) => {
                    error!("sdk-bridge: error when sending a message: {:?}", err);
                    break;
                },
            }
        }
    });

    (msg_tx, request_task, arbitrary_msg_task)
}

async fn send_json_msg<J>(
    socket_tx: &mpsc::Sender<tungstenite::protocol::Message>,
    msg: &J,
) -> Result<(), String>
where
    J: ?Sized + serde::ser::Serialize,
{
    match serde_json::to_string(msg) {
        Ok(msg) => match socket_tx
            .send(tungstenite::protocol::Message::Text(msg.into()))
            .await
        {
            Ok(_) => Ok(()),
            Err(err) => {
                error!("sdk-bridge: error when sending a message: {:?}", err);
                Err("broken connection with the gateway".to_string())
            },
        },
        Err(err) => {
            let err =
                format!("error when serializing request to JSON (this will never happen): {err:?}");
            error!("sdk-bridge: {}", err);
            Err(err)
        },
    }
}

fn error_response(request_id: RequestId, code: u16, msg: String) -> JsonResponse {
    JsonResponse {
        id: request_id,
        code,
        header: vec![],
        body_base64: msg,
    }
}
