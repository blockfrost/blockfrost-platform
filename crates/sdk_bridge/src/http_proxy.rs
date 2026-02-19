use crate::protocol::{JsonHeader, JsonRequest, JsonRequestMethod, JsonResponse, RequestId};
use crate::ws_client::{BridgeError, BridgeHandle};
use axum::body::Body;
use axum::http::{Method, Request, StatusCode};
use axum::response::{IntoResponse, Response};
use axum::{Extension, Router};
use std::net::SocketAddr;
use tracing::{error, warn};

const MAX_BODY_BYTES: usize = 1024 * 1024;

#[derive(Clone)]
struct ProxyState {
    bridge: BridgeHandle,
}

pub async fn serve(addr: SocketAddr, bridge: BridgeHandle) -> anyhow::Result<()> {
    let app = Router::new()
        .fallback(proxy_route)
        .layer(Extension(ProxyState { bridge }));

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;
    Ok(())
}

async fn proxy_route(
    Extension(state): Extension<ProxyState>,
    req: Request<Body>,
) -> impl IntoResponse {
    let json_req = match request_to_json(req).await {
        Ok(req) => req,
        Err((code, reason)) => {
            error!("sdk-bridge: request conversion error: {}: {}", code, reason);
            return (code, reason).into_response();
        },
    };

    match state.bridge.hydra().try_reserve_credit() {
        Ok(()) => (),
        Err(crate::hydra_client::CreditError::InsufficientCredits) => {
            return (StatusCode::PAYMENT_REQUIRED, "Prepaid credits exhausted").into_response();
        },
    }

    let response = match state.bridge.forward_request(json_req).await {
        Ok(resp) => resp,
        Err(err) => return bridge_error_to_response(err),
    };

    if (200..400).contains(&response.code) {
        state.bridge.hydra().account_one_request().await;
    }

    match json_to_response(response).await {
        Ok(resp) => resp.into_response(),
        Err((code, reason)) => {
            warn!(
                "sdk-bridge: response conversion error: {}: {}",
                code, reason
            );
            (code, reason).into_response()
        },
    }
}

fn bridge_error_to_response(err: BridgeError) -> Response {
    match err {
        BridgeError::ConnectionClosed => (
            StatusCode::SERVICE_UNAVAILABLE,
            "Gateway WebSocket is not available",
        )
            .into_response(),
        BridgeError::Timeout => (
            StatusCode::GATEWAY_TIMEOUT,
            "Gateway WebSocket request timed out",
        )
            .into_response(),
        BridgeError::ResponseDropped => (
            StatusCode::BAD_GATEWAY,
            "Gateway WebSocket dropped the response",
        )
            .into_response(),
    }
}

async fn request_to_json(request: Request<Body>) -> Result<JsonRequest, (StatusCode, String)> {
    let method = match request.method() {
        &Method::GET => JsonRequestMethod::GET,
        &Method::POST => JsonRequestMethod::POST,
        other => {
            return Err((
                StatusCode::BAD_REQUEST,
                format!("unhandled request method: {other}"),
            ));
        },
    };

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

    let path = request
        .uri()
        .path_and_query()
        .map(|p| p.as_str())
        .unwrap_or_else(|| request.uri().path())
        .to_string();

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
        id: RequestId(uuid::Uuid::new_v4()),
        path,
        method,
        body_base64,
        header,
    })
}

async fn json_to_response(json: JsonResponse) -> Result<Response, (StatusCode, String)> {
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
                            format!("Invalid base64 encoding of response body_base64: {err}"),
                        )
                    })?;
            Body::from(body_bytes)
        }
    };

    let mut rv = Response::builder().status(StatusCode::from_u16(json.code).map_err(|err| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Invalid response status code {}: {}", json.code, err),
        )
    })?);

    for h in json.header {
        rv = rv.header(h.name, h.value);
    }

    rv.body(body).map_err(|err| {
        (
            StatusCode::BAD_GATEWAY,
            format!("Error when constructing a response: {err}"),
        )
    })
}
