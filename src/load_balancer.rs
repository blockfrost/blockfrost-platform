use crate::errors::{AppError, BlockfrostError};
use crate::server::ApiPrefix;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::Mutex;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;
use tracing::{error, info, warn};
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LoadBalancerConfig {
    pub uri: String,
    pub access_token: String,
}

/// It’s slightly less than on the server side to desynchronize
/// [`LoadBalancerMessage::Ping`] with [`RelayMessage::Ping`]:.
const WS_PING_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1 * 13);

/// How long we allow Axum to work on a [`JsonRequest`].
const REQUEST_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(1 * 60);

const MAX_BODY_BYTES: usize = 1 * 1024 * 1024;

/// Whenever a single load balancer connection breaks, we abort all of them.
/// This logic will have to be better once we actually use more than a single
/// connection for high availability.
pub async fn run_all(
    configs: Vec<LoadBalancerConfig>,
    http_router: axum::Router,
    health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
    api_prefix: ApiPrefix,
) {
    let connections: Vec<JoinHandle<Result<(), String>>> = configs
        .iter()
        .map(|c| {
            tokio::spawn(event_loop::run(
                c.clone(),
                http_router.clone(),
                health_errors.clone(),
                api_prefix.clone(),
            ))
        })
        .collect();

    // Now wait for the first (only?) one to finish, and abort all others, and join all.
    let (first_res, first_idx, remaining) = futures::future::select_all(connections).await;

    let maybe_error = match first_res {
        Err(panic) => format!(" with a panic: {:?}", panic),
        Ok(Err(err)) => format!(" with an error: {}", err),
        Ok(Ok(())) => "".to_string(),
    };

    *health_errors.lock().await = vec![
        AppError::LoadBalancer(format!(
            "Load balancer connection ended unexpectedly{}",
            maybe_error
        ))
        .into(),
    ];

    error!(
        "load balancer: connection to {} finished{}",
        configs[first_idx].uri, maybe_error
    );

    warn!(
        "load balancer: aborting the remaining {} connection(s)",
        remaining.len()
    );
    for r in remaining.iter() {
        r.abort();
    }

    let _ignored_failure = futures::future::join_all(remaining).await;
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub struct RequestId(Uuid);

#[derive(Serialize, Deserialize, Debug)]
struct JsonRequest {
    id: RequestId,
    method: JsonRequestMethod,
    path: String,
    header: Vec<JsonHeader>,
    body_base64: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct JsonResponse {
    id: RequestId,
    code: u16,
    header: Vec<JsonHeader>,
    body_base64: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct JsonHeader {
    name: String,
    value: String,
}

#[derive(Serialize, Deserialize, Debug)]
enum JsonRequestMethod {
    GET,
    POST,
}

impl JsonRequestMethod {
    pub fn as_str(&self) -> &'static str {
        match self {
            JsonRequestMethod::GET => "GET",
            JsonRequestMethod::POST => "POST",
        }
    }
}

/// The WebSocket messages that we receive.
#[derive(Serialize, Deserialize, Debug)]
enum LoadBalancerMessage {
    Request(JsonRequest),
    Ping(u64),
    Pong(u64),
}

/// The WebSocket messages that we send.
#[derive(Serialize, Deserialize, Debug)]
enum RelayMessage {
    Response(JsonResponse),
    Ping(u64),
    Pong(u64),
}

mod event_loop {
    use super::*;
    use futures::stream::{SplitSink, SplitStream};
    use futures_util::{SinkExt, StreamExt};
    use tungstenite::protocol::Message;

    /// For clarity, let’s have a single connection 'event_loop per WebSocket
    /// connection, with the following events:
    enum LBEvent {
        NewLoadBalancerMessage(LoadBalancerMessage),
        NewResponse(JsonResponse),
        PingTick,
        SocketError(String),
    }

    /// Top-level logic of a single WebSocket connection with a load balancer.
    pub async fn run(
        config: LoadBalancerConfig,
        http_router: axum::Router,
        health_errors: Arc<Mutex<Vec<BlockfrostError>>>,
        api_prefix: ApiPrefix,
    ) -> Result<(), String> {
        let (mut socket_tx, socket_rx) = connect(config.clone()).await?.split();
        *health_errors.lock().await = vec![];

        let (event_tx, mut event_rx) = mpsc::channel::<LBEvent>(64);
        let request_task = wire_requests(event_tx.clone(), socket_rx, config.clone()).await;

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

        // Event loop state (let’s keep it minimal, please):
        let mut last_ping_sent_at: Option<std::time::Instant> = None;
        let mut last_ping_id: u64 = 0;
        let mut loop_error: Result<(), String> = Ok(());

        // Schedule the first `PingTick` immediately, otherwise we won’t start
        // checking for ping timeout:
        schedule_ping_tick();

        // The actual connection event loop:
        'event_loop: while let Some(msg) = event_rx.recv().await {
            match msg {
                LBEvent::SocketError(err) => {
                    loop_error = Err(err);
                    break 'event_loop;
                },

                LBEvent::NewLoadBalancerMessage(LoadBalancerMessage::Request(request)) => {
                    let router = http_router.clone(); // cheap, and Axum also does it for each request
                    let event_tx = event_tx.clone();
                    let api_prefix = api_prefix.clone();
                    tokio::spawn(async move {
                        let response = handle_one(router, request, api_prefix).await;
                        let _ignored_failure: Result<_, _> =
                            event_tx.send(LBEvent::NewResponse(response)).await;
                    });
                },

                LBEvent::NewResponse(response) => {
                    if let Err(err) =
                        send_json_msg(&mut socket_tx, &RelayMessage::Response(response), &config)
                            .await
                    {
                        loop_error = Err(err);
                        break 'event_loop;
                    }
                },

                LBEvent::NewLoadBalancerMessage(LoadBalancerMessage::Ping(ping_id)) => {
                    if let Err(err) =
                        send_json_msg(&mut socket_tx, &RelayMessage::Pong(ping_id), &config).await
                    {
                        loop_error = Err(err);
                        break 'event_loop;
                    }
                },

                LBEvent::NewLoadBalancerMessage(LoadBalancerMessage::Pong(pong_id)) => {
                    if pong_id == last_ping_id {
                        last_ping_sent_at = None;
                    }
                },

                LBEvent::PingTick => {
                    if let Some(_sent_at) = last_ping_sent_at {
                        // Ping timeout:
                        loop_error = Err("ping timeout".to_string());
                        break 'event_loop;
                    } else {
                        // The periodic `PingTick` loop:
                        schedule_ping_tick();
                        // Time to send a new ping:
                        last_ping_id += 1;
                        last_ping_sent_at = Some(std::time::Instant::now());
                        if let Err(_) = send_json_msg(
                            &mut socket_tx,
                            &LoadBalancerMessage::Ping(last_ping_id),
                            &config,
                        )
                        .await
                        {
                            break 'event_loop;
                        }
                    }
                },
            }
        }

        // Wait for all children to finish:
        let children = [request_task];
        children.iter().for_each(|t| t.abort());
        futures::future::join_all(children).await;

        loop_error
    }

    async fn connect(
        config: LoadBalancerConfig,
    ) -> Result<
        tokio_tungstenite::WebSocketStream<
            tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
        >,
        String,
    > {
        use tungstenite::client::IntoClientRequest;
        info!("load balancer: connecting to {}", config.uri);
        let mut request = config.uri.clone().into_client_request().unwrap();
        request.headers_mut().insert(
            "Authorization",
            format!("Bearer {}", config.access_token).parse().unwrap(),
        );
        let (ws_stream, _response) = tokio_tungstenite::connect_async(request)
            .await
            .map_err(|err| err.to_string())?;
        info!("load balancer: connected to {}", config.uri);
        Ok(ws_stream)
    }

    /// Sends a JSON message to a WebSocket. `Err(_)` is returned when you
    /// need to break the 'event_loop, because the connection is already broken.
    async fn send_json_msg<J>(
        socket_tx: &mut SplitSink<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
            Message,
        >,
        msg: &J,
        config: &LoadBalancerConfig,
    ) -> Result<(), String>
    where
        J: ?Sized + serde::ser::Serialize,
    {
        match serde_json::to_string(msg) {
            Ok(msg) => {
                match socket_tx.send(Message::Text(msg.into())).await {
                    Ok(_) => Ok(()),
                    Err(err) => {
                        error!(
                            "load balancer: {}: error when sending a Pong: {:?}",
                            config.uri, err
                        );
                        // Something wrong with the socket, let’s break the 'event_loop:
                        Err("broken connection with the load balancer".to_string())
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
                error!("load balancer: {}: {}", config.uri, err);
                Err(err)
            },
        }
    }

    async fn wire_requests(
        event_tx: mpsc::Sender<LBEvent>,
        mut socket_rx: SplitStream<
            tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        >,
        config: LoadBalancerConfig,
    ) -> JoinHandle<()> {
        let task = tokio::spawn(async move {
            'read_loop: loop {
                match socket_rx.next().await {
                    None => {
                        let _ignored_failure: Result<_, _> = event_tx
                            .send(LBEvent::SocketError("connection closed".to_string()))
                            .await;
                        break 'read_loop;
                    },
                    Some(Err(err)) => {
                        let _ignored_failure: Result<_, _> = event_tx
                            .send(LBEvent::SocketError(format!("stream error: {:?}", err)))
                            .await;
                        break 'read_loop;
                    },
                    Some(Ok(Message::Close(frame))) => {
                        warn!(
                            "load balancer: {}: relay disconnected (CloseFrame: {:?})",
                            config.uri, frame,
                        );
                        let _ignored_failure: Result<_, _> = event_tx
                            .send(LBEvent::SocketError("relay disconnected".to_string()))
                            .await;
                        break 'read_loop;
                    },
                    Some(Ok(Message::Frame(_) | Message::Ping(_) | Message::Pong(_))) => {}, // ignore, they’re handled by the library
                    Some(Ok(Message::Binary(bin))) => {
                        warn!(
                            "load balancer: {}: received unexpected binary message: {:?}",
                            config.uri,
                            hex::encode(bin),
                        );
                    },
                    Some(Ok(Message::Text(text))) => {
                        match serde_json::from_str::<LoadBalancerMessage>(&text) {
                            Ok(msg) => {
                                if let Err(_) =
                                    event_tx.send(LBEvent::NewLoadBalancerMessage(msg)).await
                                {
                                    break 'read_loop;
                                }
                            },
                            Err(err) => warn!(
                                "load balancer: {}: received unparsable text message: {:?}: {:?}",
                                config.uri, text, err,
                            ),
                        };
                    },
                }
            }
        });
        task
    }

    /// Passes one [`JsonRequest`] through our underlying original HTTP server.
    /// Everything happens internally, in memory, without opening new TCP
    /// connections etc. – very light.
    async fn handle_one(
        http_router: axum::Router,
        request: JsonRequest,
        api_prefix: ApiPrefix,
    ) -> JsonResponse {
        use axum::body::Body;
        use hyper::StatusCode;
        use hyper::{Request, Response};
        use tower::ServiceExt;

        let request_id = request.id.clone();
        let request_id_ = request.id.clone();

        let rv: Result<JsonResponse, (StatusCode, String)> = async {
            let req: Request<Body> = json_to_request(request, api_prefix)?;

            let response: Response<Body> =
                tokio::time::timeout(REQUEST_TIMEOUT, http_router.into_service().oneshot(req))
                    .await
                    .map_err(|_elapsed| {
                        (
                            StatusCode::GATEWAY_TIMEOUT,
                            format!(
                                "Timed out while waiting {:?} for a response",
                                REQUEST_TIMEOUT
                            ),
                        )
                    })?
                    .unwrap(); // unwrap is safe, because the error is a non-instantiable [`std::convert::Infallible`]

            response_to_json(response, request_id).await
        }
        .await;

        match rv {
            Ok(ok) => ok,
            Err((code, err)) => {
                error!("load balancer: returning {}, because: {}", code, err);
                JsonResponse {
                    id: request_id_,
                    code: code.into(),
                    header: vec![],
                    body_base64: err,
                }
            },
        }
    }
}

fn json_to_request(
    json: JsonRequest,
    api_prefix: ApiPrefix,
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
                            format!("Invalid base64 encoding of body_base64: {}", err),
                        )
                    })?;
            Body::from(body_bytes)
        }
    };

    let mut rv = Request::builder()
        .method(json.method.as_str())
        .uri(format!("{}{}", api_prefix, json.path));

    for h in json.header {
        rv = rv.header(h.name, h.value);
    }

    rv.body(body).map_err(|err| {
        (
            StatusCode::BAD_REQUEST,
            format!(
                "Error when constructing a request from JSON request: {}",
                err
            ),
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
                    format!("Cannot read body of the response: {}", err),
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
