use crate::blockfrost::AssetName;
use crate::errors::APIError;
use crate::hydra;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::{Arc, atomic};
use tokio::sync::{Mutex, mpsc, oneshot};
use tokio::task::JoinHandle;
use tracing::{error, info, warn};
use uuid::Uuid;

const ACCESS_TOKEN_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5 * 60);
const MAX_BODY_BYTES: usize = 1024 * 1024;
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(60);
const WS_PING_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(15);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct AccessToken(pub String);

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RequestId(Uuid);

#[derive(Serialize, Deserialize, Debug)]
pub struct JsonRequest {
    pub id: RequestId,
    method: JsonRequestMethod,
    path: String,
    pub header: Vec<JsonHeader>,
    body_base64: String,
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
    name: String,
    value: String,
}

#[allow(clippy::upper_case_acronyms)]
#[derive(Serialize, Deserialize, Debug)]
pub enum JsonRequestMethod {
    GET,
    POST,
}

/// The WebSocket messages that we send.
#[derive(Serialize, Deserialize, Debug)]
pub enum LoadBalancerMessage {
    Request(JsonRequest),
    HydraKExResponse(hydra::KeyExchangeResponse),
    HydraTunnel { connection_id: u64, bytes: Vec<u8> },
    Ping(u64),
    Pong(u64),
}

/// The WebSocket messages that we receive.
#[derive(Serialize, Deserialize, Debug)]
pub enum RelayMessage {
    Response(JsonResponse),
    HydraKExRequest(hydra::KeyExchangeRequest),
    HydraTunnel { connection_id: u64, bytes: Vec<u8> },
    Ping(u64),
    Pong(u64),
}

#[derive(Clone, Debug)]
pub struct LoadBalancerState {
    pub access_tokens: Arc<Mutex<HashMap<AccessToken, AccessTokenState>>>,
    pub active_relays: Arc<Mutex<HashMap<Uuid, RelayState>>>,
    pub background_worker: Arc<JoinHandle<()>>,
}

#[derive(Debug)]
pub struct AccessTokenState {
    pub name: AssetName,
    pub api_prefix: Uuid,
    pub expires: std::time::Instant,
}

#[derive(Clone, Debug)]
pub struct RelayState {
    pub name: AssetName,
    pub new_request_channel: mpsc::Sender<RequestState>,
    /// Send this to end the event loop of the connection, and disconnect the
    /// relay, with the [`String`] as the reason. It’s a little controversial
    /// for this to be an MPSC, but also the cleanest. You can’t clone [`oneshot`].
    pub do_finish: mpsc::Sender<String>,
    pub requests_in_progress: Arc<Mutex<HashMap<RequestId, RequestState>>>,
    pub network_rtt: Arc<Mutex<Option<std::time::Duration>>>,
    pub connected_since: std::time::Instant,
    pub requests_sent: Arc<atomic::AtomicU64>,
    pub responses_received: Arc<atomic::AtomicU64>,
}

#[derive(Debug)]
pub struct RequestState {
    respond_to: oneshot::Sender<JsonResponse>,
    expires: std::time::Instant, // never read, do we need it?
    underlying: JsonRequest,
}

impl LoadBalancerState {
    pub async fn new() -> LoadBalancerState {
        let access_tokens = Arc::new(Mutex::new(HashMap::new()));
        let active_relays = Arc::new(Mutex::new(HashMap::new()));
        let background_worker = Arc::new(tokio::spawn(Self::clean_up_expired_tokens_periodically(
            access_tokens.clone(),
        )));

        LoadBalancerState {
            access_tokens,
            active_relays,
            background_worker,
        }
    }

    pub async fn new_access_token(&self, name: AssetName, api_prefix: Uuid) -> AccessToken {
        let expires = std::time::Instant::now() + ACCESS_TOKEN_TIMEOUT;
        let token = random_token();
        self.access_tokens.lock().await.insert(
            token.clone(),
            AccessTokenState {
                name,
                api_prefix,
                expires,
            },
        );
        token
    }

    pub async fn register(&self, token: &str) -> Result<AccessTokenState, APIError> {
        let token = AccessToken(token.to_string());
        let state = self
            .access_tokens
            .lock()
            .await
            .remove(&token)
            .ok_or(APIError::Unauthorized())?;
        if state.expires < std::time::Instant::now() {
            Err(APIError::Unauthorized())?;
        }
        Ok(state)
    }

    async fn clean_up_expired_tokens_periodically(
        access_tokens: Arc<Mutex<HashMap<AccessToken, AccessTokenState>>>,
    ) {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(60)).await;
            Self::clean_up_expired_tokens(&access_tokens).await;
        }
    }

    async fn clean_up_expired_tokens(
        access_tokens: &Arc<Mutex<HashMap<AccessToken, AccessTokenState>>>,
    ) {
        let now = std::time::Instant::now();

        access_tokens.lock().await.retain(|_, state| {
            let still_valid = state.expires > now;
            if !still_valid {
                warn!(
                    "load balancer: {}: unused WebSocket access token expired",
                    state.name.as_str(),
                )
            }
            still_valid
        });
    }
}

/// Cancels its background tasks, when all clones of a particular [`LoadBalancerState`] go out of scope.
impl Drop for LoadBalancerState {
    fn drop(&mut self) {
        // Abort the background task, if this is the _last_ clone of [`LoadBalancerState`]:
        if Arc::strong_count(&self.background_worker) == 1 {
            self.background_worker.abort();
        }
    }
}

/// Generates a random Base64-encoded string. Used for generating access tokens.
pub fn random_token() -> AccessToken {
    use base64::{Engine as _, engine::general_purpose};
    use rand::RngCore;
    let mut bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut bytes);
    let token = general_purpose::STANDARD.encode(bytes);
    AccessToken(token)
}

/// The HTTP (incl. WebSocket) endpoints that the load balancer exposes.
pub mod api {
    use super::*;
    use crate::errors::APIError;
    use axum::{
        Extension,
        extract::{Path, Request, WebSocketUpgrade},
        http::{HeaderMap, StatusCode},
        response::IntoResponse,
    };
    use tokio::sync::oneshot;
    use uuid::Uuid;

    /// The main WebSocker route that will be upgraded to a proper WebSocket
    /// connection by capable clients.
    pub async fn websocket_route(
        ws: WebSocketUpgrade,
        Extension(load_balancer): Extension<LoadBalancerState>,
        headers: HeaderMap,
    ) -> Result<impl IntoResponse, APIError> {
        let token: String = headers
            .get("Authorization")
            .and_then(|a| a.to_str().ok())
            .and_then(|a| a.strip_prefix("Bearer "))
            .ok_or(APIError::Unauthorized())?
            .to_string();
        let token_state = load_balancer.register(&token).await?;
        Ok(ws.on_upgrade(|socket| event_loop::run(load_balancer, token_state, socket)))
    }

    /// Axum noise, a proxy to `handle_prefix_route`.
    pub async fn prefix_route_root(
        Path(uuid): Path<String>,
        Extension(load_balancer): Extension<LoadBalancerState>,
        req: Request,
    ) -> Result<impl IntoResponse, APIError> {
        handle_prefix_route(load_balancer, uuid, "/".to_string(), req).await
    }

    /// Axum noise, a proxy to `handle_prefix_route`.
    pub async fn prefix_route(
        Path((uuid, rest)): Path<(String, String)>,
        Extension(load_balancer): Extension<LoadBalancerState>,
        req: Request,
    ) -> Result<impl IntoResponse, APIError> {
        handle_prefix_route(load_balancer, uuid, format!("/{}", rest), req).await
    }

    #[derive(Serialize, Deserialize, Debug)]
    struct RelayStats {
        api_prefix: Uuid,
        network_rtt_seconds: Option<f64>,
        connected_since: chrono::DateTime<chrono::Utc>,
        requests_sent: u64,
        responses_received: u64,
        requests_in_progress: u64,
    }

    /// This route shows some stats about all relays connected with a WebSocket,
    /// and their RTT (round-trip time).
    pub async fn stats_route(
        Extension(load_balancer): Extension<LoadBalancerState>,
    ) -> impl IntoResponse {
        let mut rv: HashMap<AssetName, RelayStats> = HashMap::new();
        let now_chrono = chrono::Utc::now();
        let now_instant = std::time::Instant::now();

        for (api_prefix, relay_state) in load_balancer.active_relays.lock().await.iter() {
            rv.insert(
                relay_state.name.clone(),
                RelayStats {
                    api_prefix: *api_prefix,
                    network_rtt_seconds: relay_state
                        .network_rtt
                        .lock()
                        .await
                        .map(|a| a.as_secs_f64()),
                    connected_since: now_chrono - (now_instant - relay_state.connected_since),
                    requests_sent: relay_state.requests_sent.load(atomic::Ordering::SeqCst),
                    responses_received: relay_state
                        .responses_received
                        .load(atomic::Ordering::SeqCst),
                    requests_in_progress: relay_state.requests_in_progress.lock().await.len()
                        as u64,
                },
            );
        }

        axum::Json(rv)
    }

    /// This route handles requests directed at particular relays. For now, we
    /// allow end users to specify which relay they want with a UUID prefix.
    ///
    /// E.g. `GET /{uuid}/metrics` will be translated to a WebSocket message,
    /// and handled by the relay identified by said `{uuid}`.
    async fn handle_prefix_route(
        load_balancer: LoadBalancerState,
        uuid: String,
        rest: String,
        req: Request,
    ) -> Result<impl IntoResponse, APIError> {
        let rv: Result<hyper::Response<axum::body::Body>, (StatusCode, String)> = async move {
            let api_prefix = Uuid::parse_str(&uuid).map_err(|_| {
                (
                    StatusCode::NOT_FOUND,
                    format!("unparsable UUID prefix: {}", uuid),
                )
            })?;

            let (new_request_channel, relay_name): (mpsc::Sender<RequestState>, AssetName) =
                load_balancer
                    .active_relays
                    .lock()
                    .await
                    .get(&api_prefix)
                    .ok_or_else(|| {
                        (
                            StatusCode::NOT_FOUND,
                            format!("relay {} not found for request: {}", api_prefix, rest),
                        )
                    })
                    .map(|rs| (rs.new_request_channel.clone(), rs.name.clone()))?;

            let json_req = request_to_json(req, rest.clone(), &relay_name).await?;

            let (response_tx, response_rx) = oneshot::channel::<JsonResponse>();

            let new_request = RequestState {
                expires: std::time::Instant::now() + REQUEST_TIMEOUT,
                respond_to: response_tx,
                underlying: json_req,
            };

            new_request_channel.send(new_request).await.map_err(|_| {
                (
                    StatusCode::BAD_GATEWAY,
                    format!(
                        "receiver {} dropped; request: {}",
                        relay_name.as_str(),
                        rest
                    ),
                )
            })?;

            match tokio::time::timeout(REQUEST_TIMEOUT, response_rx).await {
                Ok(Ok(response)) => json_to_response(response, &relay_name).await,
                Ok(Err(_)) => {
                    // sender dropped
                    Err((
                        StatusCode::BAD_GATEWAY,
                        format!(
                            "relay {} dropped while awaiting response for: {}",
                            relay_name.as_str(),
                            rest
                        ),
                    ))
                },
                Err(_) => Err((
                    StatusCode::GATEWAY_TIMEOUT,
                    format!("relay {} timed out for: {}", relay_name.as_str(), rest),
                )),
            }
        }
        .await;

        match rv {
            Ok(resp) => Ok(resp),
            Err((code, reason)) => {
                error!("load balancer: returning {}, because: {}", code, reason);
                Ok((code, reason).into_response())
            },
        }
    }
}

/// The WebSocket event loop, passing messages between HTTP<->WebSocket, keeping
/// track of persistent connection liveness, etc.
pub mod event_loop {
    use super::*;
    use axum::extract::ws::{Message, WebSocket};
    use axum::http::StatusCode;
    use futures_util::{
        sink::SinkExt,
        stream::{SplitSink, StreamExt},
    };

    /// For clarity, let’s have a single connection 'event_loop per WebSocket
    /// connection, with the following events:
    enum LBEvent {
        NewRequest(RequestState),
        NewRelayMessage(RelayMessage),
        PingTick,
        Finish(String),
    }

    /// Top-level logic of a single WebSocket connection with a relay.
    pub async fn run(
        load_balancer: LoadBalancerState,
        token_state: AccessTokenState,
        socket: WebSocket,
    ) {
        let asset_name = &token_state.name;

        // Allow only 1 connection per NFT:
        disconnect_existing_sessions_of(&token_state, &load_balancer).await;

        info!(
            "load balancer: {}: new relay connection",
            asset_name.as_str()
        );

        let (event_tx, mut event_rx) = mpsc::channel::<LBEvent>(64);
        let (request_tx, request_task) = wire_requests(event_tx.clone()).await;
        let (finish_tx, finish_task) = wire_do_finish(event_tx.clone()).await;
        let (mut socket_tx, response_task) =
            wire_responses(event_tx.clone(), socket, asset_name).await;

        let relay_state = RelayState {
            name: token_state.name.clone(),
            new_request_channel: request_tx,
            do_finish: finish_tx,
            requests_in_progress: Arc::new(Mutex::new(HashMap::new())),
            network_rtt: Arc::new(Mutex::new(None)),
            connected_since: std::time::Instant::now(),
            requests_sent: Arc::new(atomic::AtomicU64::new(0)),
            responses_received: Arc::new(atomic::AtomicU64::new(0)),
        };

        let clean_up_task = tokio::spawn(clean_up_expired_requests_periodically(
            relay_state.requests_in_progress.clone(),
        ));

        load_balancer
            .active_relays
            .lock()
            .await
            .insert(token_state.api_prefix, relay_state.clone());

        let schedule_ping_tick = {
            let event_tx = event_tx.clone();
            move || {
                let tx = event_tx.clone();
                tokio::spawn(async move {
                    tokio::time::sleep(WS_PING_TIMEOUT).await;
                    let _ignored_failure: Result<_, _> = tx.send(LBEvent::PingTick).await;
                })
            }
        };

        // Schedule the first `PingTick` immediately, otherwise we won’t start
        // checking for ping timeout, and won’t measure the network RTT:
        let _ignored_failure: Result<_, _> = event_tx.send(LBEvent::PingTick).await;

        // Event loop state (let’s keep it minimal, please):
        let mut last_ping_sent_at: Option<std::time::Instant> = None;
        let mut last_ping_id: u64 = 0;
        let mut disconnection_reason = None;

        // The actual connection event loop:
        'event_loop: while let Some(msg) = event_rx.recv().await {
            match msg {
                LBEvent::Finish(reason) => {
                    disconnection_reason = Some(reason);
                    break 'event_loop;
                },

                LBEvent::NewRequest(request) => {
                    if pass_on_request(request, &relay_state, asset_name, &mut socket_tx)
                        .await
                        .is_err()
                    {
                        break 'event_loop;
                    }
                },

                LBEvent::NewRelayMessage(RelayMessage::HydraTunnel { .. }) => {
                    todo!()
                },

                LBEvent::NewRelayMessage(RelayMessage::HydraKExRequest { .. }) => {
                    // FIXME: actually exchange
                    let resp = crate::hydra::fake_kex_response(&crate::types::Network::Preview)
                        .await
                        .unwrap();
                    if send_json_msg(
                        &mut socket_tx,
                        &LoadBalancerMessage::HydraKExResponse(resp),
                        asset_name,
                    )
                    .await
                    .is_err()
                    {
                        break 'event_loop;
                    }
                },

                LBEvent::NewRelayMessage(RelayMessage::Response(response)) => {
                    pass_on_response(response, &relay_state, asset_name).await;
                },

                LBEvent::NewRelayMessage(RelayMessage::Ping(ping_id)) => {
                    if send_json_msg(
                        &mut socket_tx,
                        &LoadBalancerMessage::Pong(ping_id),
                        asset_name,
                    )
                    .await
                    .is_err()
                    {
                        break 'event_loop;
                    }
                },

                LBEvent::NewRelayMessage(RelayMessage::Pong(pong_id)) => {
                    if pong_id == last_ping_id {
                        if let Some(sent_at) = last_ping_sent_at {
                            let network_rtt = sent_at.elapsed();
                            *(relay_state.network_rtt.lock().await) = Some(network_rtt);
                        }
                        last_ping_sent_at = None;
                    }
                },

                LBEvent::PingTick => {
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
                        if send_json_msg(
                            &mut socket_tx,
                            &LoadBalancerMessage::Ping(last_ping_id),
                            asset_name,
                        )
                        .await
                        .is_err()
                        {
                            break 'event_loop;
                        }
                    }
                },
            }
        }

        let disconnection_reason_ = disconnection_reason
            .clone()
            .unwrap_or("reason unknown".to_string());

        warn!(
            "load balancer: {}: connection event loop finished: {}",
            asset_name.as_str(),
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

        // Stop ingress of new requests to this already broken connection by
        // deleting its producer (`request_tx`) from the `LoadBalancerState`:
        load_balancer
            .active_relays
            .lock()
            .await
            .remove(&token_state.api_prefix);

        // Fail all remaining requests for this relay that possibly are still on
        // the channel after `break 'event_loop`.
        while let Ok(msg) = event_rx.try_recv() {
            match msg {
                LBEvent::NewRequest(request) => {
                    fail_request(
                        request,
                        StatusCode::BAD_GATEWAY,
                        &format!(
                            "relay disconnected with pending requests: {}",
                            disconnection_reason_
                        ),
                        asset_name,
                    )
                    .await;
                },

                // It’s also possible that some responses are pending, it’s best
                // to pass them on:
                LBEvent::NewRelayMessage(RelayMessage::Response(response)) => {
                    pass_on_response(response, &relay_state, asset_name).await;
                },

                _ => (), // ignore any other pending event
            }
        }

        drop(event_rx);

        // Fail all in-progress requests for this relay:
        for (_, request) in relay_state.requests_in_progress.lock().await.drain() {
            fail_request(
                request,
                StatusCode::BAD_GATEWAY,
                &format!(
                    "relay disconnected with in-progress requests: {}",
                    disconnection_reason_
                ),
                asset_name,
            )
            .await;
        }

        // Wait for all children to finish:
        let children = [request_task, finish_task, response_task, clean_up_task];
        children.iter().for_each(|t| t.abort());
        futures::future::join_all(children).await;

        info!("load balancer: {}: lost relay", asset_name.as_str());
    }

    /// We currently want to allow only a single connection per NFT:
    async fn disconnect_existing_sessions_of(
        token_state: &AccessTokenState,
        load_balancer: &LoadBalancerState,
    ) {
        let mut other_do_finish_tx: Vec<mpsc::Sender<String>> = Vec::with_capacity(1);
        load_balancer
            .active_relays
            .lock()
            .await
            .retain(|_other_api_prefix, other_relay_state| {
                if other_relay_state.name == token_state.name {
                    other_do_finish_tx.push(other_relay_state.do_finish.clone());
                    false
                } else {
                    true
                }
            });
        for chan in other_do_finish_tx.iter() {
            let _ignored_failure: Result<_, _> = chan
                .send(format!(
                    "overridden by the latest {} registration with prefix {}",
                    token_state.name.as_str(),
                    token_state.api_prefix
                ))
                .await;
        }
    }

    /// A background task to periodically remove timed-out requests from
    /// `requests_in_progress`. It matters only for conserving memory, no other
    /// logic depends on it.
    async fn clean_up_expired_requests_periodically(
        requests_in_progress: Arc<Mutex<HashMap<RequestId, RequestState>>>,
    ) {
        use std::time::{Duration, Instant};
        let safety_margin = Duration::from_secs(10);
        loop {
            tokio::time::sleep(Duration::from_secs(60)).await;
            let now = Instant::now();
            requests_in_progress
                .lock()
                .await
                .retain(|_, req| req.expires + safety_margin > now);
        }
    }

    /// A little dance around the borrow checker to avoid cloning the request
    /// for serialization.
    fn serialize_request(
        request: RequestState,
    ) -> (
        RequestState,
        Result<serde_json::Value, serde_json::error::Error>,
    ) {
        let mut request = request;
        let msg = LoadBalancerMessage::Request(request.underlying);
        let json = serde_json::to_value(&msg);
        let LoadBalancerMessage::Request(underlying) = msg else {
            unreachable!()
        };
        request.underlying = underlying;
        (request, json)
    }

    /// Wire HTTP requests to the connection 'event_loop:
    async fn wire_requests(
        event_tx: mpsc::Sender<LBEvent>,
    ) -> (mpsc::Sender<RequestState>, JoinHandle<()>) {
        let (tx, mut rx) = mpsc::channel(64);
        let task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if event_tx.send(LBEvent::NewRequest(msg)).await.is_err() {
                    break;
                }
            }
        });
        (tx, task)
    }

    /// Wire `do_finish` signals to the connection 'event_loop:
    async fn wire_do_finish(
        event_tx: mpsc::Sender<LBEvent>,
    ) -> (mpsc::Sender<String>, JoinHandle<()>) {
        let (tx, mut rx) = mpsc::channel(64);
        let task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                if event_tx.send(LBEvent::Finish(msg)).await.is_err() {
                    break;
                }
            }
        });
        (tx, task)
    }

    /// Wire responses to the connection 'event_loop:
    async fn wire_responses(
        event_tx: mpsc::Sender<LBEvent>,
        socket: WebSocket,
        asset_name: &AssetName,
    ) -> (SplitSink<WebSocket, Message>, JoinHandle<()>) {
        let (tx, mut rx) = socket.split();
        let asset_name = asset_name.clone();
        let task = tokio::spawn(async move {
            while let Some(Ok(msg)) = rx.next().await {
                match msg {
                    Message::Text(text) => {
                        match serde_json::from_str::<RelayMessage>(&text) {
                            Ok(msg) => {
                                if event_tx.send(LBEvent::NewRelayMessage(msg)).await.is_err() {
                                    break;
                                }
                            },
                            Err(err) => warn!(
                                "load balancer: {}: received unparsable text message: {:?}: {:?}",
                                asset_name.as_str(),
                                text,
                                err,
                            ),
                        };
                    },
                    Message::Binary(bin) => {
                        warn!(
                            "load balancer: {}: received unexpected binary message: {:?}",
                            asset_name.as_str(),
                            hex::encode(bin),
                        );
                    },
                    Message::Close(frame) => {
                        warn!(
                            "load balancer: {}: relay disconnected (CloseFrame: {:?})",
                            asset_name.as_str(),
                            frame,
                        );
                        let _ignored_failure: Result<_, _> = event_tx
                            .send(LBEvent::Finish("relay disconnected".to_string()))
                            .await;
                        break;
                    },
                    Message::Ping(_) | Message::Pong(_) => {},
                }
            }
        });
        (tx, task)
    }

    /// Sends a JSON message to a WebSocket. `Err(_)` is returned when you
    /// need to break the 'event_loop, because the connection is already broken.
    async fn send_json_msg<J>(
        socket_tx: &mut SplitSink<WebSocket, Message>,
        msg: &J,
        asset_name: &AssetName,
    ) -> Result<(), String>
    where
        J: ?Sized + serde::ser::Serialize,
    {
        match serde_json::to_string(msg) {
            Ok(msg) => {
                match socket_tx.send(Message::Text(msg)).await {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        error!(
                            "load balancer: {}: error when sending a Pong: {:?}",
                            asset_name.as_str(),
                            err
                        );
                        // Something wrong with the socket, let’s break the 'event_loop:
                        Err("broken connection with the relay".to_string())
                    },
                }
            },
            Err(err) => {
                // This branch is practically impossible, but for the sake of completeness:
                // Let’s break 'event_loop, this seems the most elegant.
                let err = format!(
                    "error when serializing request to JSON (this will never happen): {:?}",
                    err
                );
                error!("load balancer: {}: {}", asset_name.as_str(), err);
                Err(err)
            },
        }
    }

    /// Passes a WebSocket response on to the original HTTP requester.
    async fn pass_on_response(
        response: JsonResponse,
        relay_state: &RelayState,
        asset_name: &AssetName,
    ) {
        let request_id = response.id.clone();

        match relay_state
            .requests_in_progress
            .lock()
            .await
            .remove(&request_id)
        {
            Some(request_state) => {
                relay_state
                    .responses_received
                    .fetch_add(1, atomic::Ordering::SeqCst);
                match request_state.respond_to.send(response) {
                    Ok(_) => (),
                    Err(_) => warn!(
                        "load balancer: {}: received response after its request timed out: {}",
                        asset_name.as_str(),
                        request_id.0,
                    ),
                }
            },
            None => warn!(
                "load balancer: {}: received supposed response for non-existent request: {}",
                asset_name.as_str(),
                response.id.0,
            ),
        }
    }

    /// Passes a HTTP request on to a WebSocket. `Err(_)` is returned when you
    /// need to break the 'event_loop.
    async fn pass_on_request(
        request: RequestState,
        relay_state: &RelayState,
        asset_name: &AssetName,
        socket_tx: &mut SplitSink<WebSocket, Message>,
    ) -> Result<(), String> {
        let request_id = request.underlying.id.clone();
        let (request, json) = serialize_request(request);
        relay_state
            .requests_in_progress
            .lock()
            .await
            .insert(request_id.clone(), request);

        let send_result = match json {
            Ok(msg) => send_json_msg(socket_tx, &msg, asset_name).await,
            Err(err) => Err(format!("error when serializing request to JSON: {:?}", err)), // impossible
        };

        match send_result {
            Ok(_) => {
                relay_state
                    .requests_sent
                    .fetch_add(1, atomic::Ordering::SeqCst);
                Ok(())
            },
            Err(err) => {
                let err = format!("error when sending request to relay: {:?}", err);

                if let Some(request) = relay_state
                    .requests_in_progress
                    .lock()
                    .await
                    .remove(&request_id)
                {
                    fail_request(request, StatusCode::BAD_REQUEST, &err, asset_name).await;
                }

                // break 'event_loop
                Err(err)
            },
        }
    }

    /// Returns a failure to the HTTP client of a given [`RequestState`].
    async fn fail_request(
        request: RequestState,
        code: StatusCode,
        why: &str,
        asset_name: &AssetName,
    ) {
        let request_id = request.underlying.id.clone();
        error!(
            "load balancer: {}: failing request with {}: {}: {:?}",
            asset_name.as_str(),
            code.to_string(),
            why,
            request.underlying,
        );
        let _ignored_failure: Result<_, _> = request
            .respond_to
            .send(JsonResponse {
                id: request_id.clone(),
                code: code.as_u16(),
                header: vec![],
                body_base64: why.to_string(),
            })
            .inspect_err(|_| {
                warn!(
                    "load balancer: {}: tried to fail a request after said request timed out: {}",
                    asset_name.as_str(),
                    request_id.0,
                )
            });
    }
}

/// Converts a [`hyper::Request`] to our [`JsonRequest`] sent over the WebSocket.
async fn request_to_json(
    request: hyper::Request<axum::body::Body>,
    path_override: String,
    relay_name: &AssetName,
) -> Result<JsonRequest, (hyper::StatusCode, String)> {
    use axum::http::{Method, StatusCode};

    let method = (match request.method() {
        &Method::GET => Ok(JsonRequestMethod::GET),
        &Method::POST => Ok(JsonRequestMethod::POST),
        other => Err((
            StatusCode::BAD_REQUEST,
            format!("unhandled request method: {}", other),
        )),
    })?;

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
                format!(
                    "failed to read body bytes for request to {}: {}: {:?}",
                    relay_name.as_str(),
                    path_override,
                    err
                ),
            )
        })?;

    use base64::{Engine as _, engine::general_purpose};
    let body_base64 = general_purpose::STANDARD.encode(body_bytes);

    Ok(JsonRequest {
        id: RequestId(Uuid::new_v4()),
        path: path_override.clone(),
        method,
        body_base64,
        header,
    })
}

/// Converts our [`JsonResponse`] sent over the Websocket to a [`hyper::Response`].
async fn json_to_response(
    json: JsonResponse,
    relay_name: &AssetName,
) -> Result<hyper::Response<axum::body::Body>, (hyper::StatusCode, String)> {
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
                        (
                            StatusCode::BAD_GATEWAY,
                            format!(
                                "{}: Invalid base64 encoding of response body_base64: {}",
                                relay_name.as_str(),
                                err
                            ),
                        )
                    })?;
            Body::from(body_bytes)
        }
    };

    let mut rv = Response::builder().status(StatusCode::from_u16(json.code).map_err(|err| {
        (
            StatusCode::BAD_GATEWAY,
            format!(
                "{}: Invalid response status code {}: {}",
                relay_name.as_str(),
                json.code,
                err
            ),
        )
    })?);

    for h in json.header {
        rv = rv.header(h.name, h.value);
    }

    rv.body(body).map_err(|err| {
        (
            StatusCode::BAD_REQUEST,
            format!(
                "{}: Error when constructing a request from JSON request: {}",
                relay_name.as_str(),
                err
            ),
        )
    })
}

#[cfg(test)]
mod tests {
    use super::*;

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
        LoadBalancerState::clean_up_expired_tokens(&lb.access_tokens).await;
        let tokens = lb.access_tokens.lock().await;

        assert_eq!(tokens.len(), 1);

        assert!(tokens.contains_key(&token_valid));
        assert!(!tokens.contains_key(&token_expired));
    }
}
