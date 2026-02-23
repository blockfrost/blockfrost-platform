use crate::BlockfrostError;
use axum::{
    body::{Bytes, to_bytes},
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use sentry::{
    Breadcrumb, Level,
    protocol::{Event, Exception},
};
use serde_json;
use std::convert::Infallible;

pub async fn error_middleware(request: Request, next: Next) -> Result<Response, Infallible> {
    let request_path = request.uri().path().to_string();
    let request_uri = request.uri().to_string();
    let response = next.run(request).await;
    let status_code = response.status();

    // Transform timeout to internal server error for user
    // 504 Gateway Timeout
    if response.status() == StatusCode::REQUEST_TIMEOUT {
        tracing::warn!(
            path = %request_path,
            uri = %request_uri,
            "Request timeout"
        );
        return Ok(BlockfrostError::internal_server_error_user().into_response());
    }

    // Transform our custom METHOD_NOT_ALLOWED err
    // to 405 status code
    if response.status() == StatusCode::METHOD_NOT_ALLOWED {
        tracing::warn!(
            path = %request_path,
            uri = %request_uri,
            "Method not allowed"
        );
        return Ok(BlockfrostError::method_not_allowed().into_response());
    }

    // Transform server errors to internal server error for user – except for 503 from the root route
    if response.status().is_server_error() && response.status() != StatusCode::SERVICE_UNAVAILABLE {
        handle_server_error(response, &request_path, &request_uri, status_code).await
    } else if response.status().is_client_error() {
        log_client_error(response, &request_path, &request_uri, status_code).await
    } else {
        Ok(response)
    }
}

async fn handle_server_error(
    response: Response,
    request_path: &str,
    request_uri: &str,
    status_code: StatusCode,
) -> Result<Response, Infallible> {
    let body = response.into_body();

    match to_bytes(body, usize::MAX).await {
        Ok(bytes) => parse_and_log_error(bytes, request_path, request_uri, status_code).await,
        Err(e) => {
            log_and_capture_error(
                "Failed to read body",
                e,
                request_path,
                request_uri,
                status_code,
            );
        },
    }

    Ok(BlockfrostError::internal_server_error_user().into_response())
}

async fn log_client_error(
    response: Response,
    request_path: &str,
    request_uri: &str,
    status_code: StatusCode,
) -> Result<Response, Infallible> {
    let (parts, body) = response.into_parts();

    match to_bytes(body, usize::MAX).await {
        Ok(bytes) => {
            let error_detail = match serde_json::from_slice::<BlockfrostError>(&bytes) {
                Ok(bf_error) => format!("{} - {}", bf_error.error, bf_error.message),
                Err(_) => String::from_utf8_lossy(&bytes).to_string(),
            };

            tracing::warn!(
                path = %request_path,
                uri = %request_uri,
                status = %status_code,
                "Client error: {}",
                error_detail,
            );

            // Reconstruct the response with the original body
            Ok(Response::from_parts(parts, axum::body::Body::from(bytes)))
        },
        Err(_) => {
            // Body read failed; return a generic error
            Ok(BlockfrostError::internal_server_error_user().into_response())
        },
    }
}

async fn parse_and_log_error(
    bytes: Bytes,
    request_path: &str,
    request_uri: &str,
    status_code: StatusCode,
) {
    match serde_json::from_slice::<BlockfrostError>(&bytes) {
        Ok(error_info) => {
            tracing::error!(
                path = %request_path,
                uri = %request_uri,
                status = %status_code,
                "Server error: {} - {}",
                error_info.error,
                error_info.message,
            );
            log_to_sentry("|", format!("{error_info:?}"), request_path, status_code)
        },
        Err(e) => {
            let body_str = String::from_utf8_lossy(&bytes);
            tracing::error!(
                path = %request_path,
                uri = %request_uri,
                status = %status_code,
                body = %body_str,
                "Server error: failed to parse body as JSON: {e:?}",
            );
            log_to_sentry(
                "JSON Parse Error",
                format!("{e:?}"),
                request_path,
                status_code,
            );
        },
    }
}

fn log_and_capture_error(
    message: &str,
    error: impl std::fmt::Debug,
    request_path: &str,
    request_uri: &str,
    status_code: StatusCode,
) {
    tracing::error!(
        path = %request_path,
        uri = %request_uri,
        status = %status_code,
        "{}: {:?}",
        message,
        error,
    );

    let exception = Exception {
        ty: "ServerError".to_string(),
        value: Some(format!("{error:?}")),
        ..Default::default()
    };

    let event = Event {
        message: Some(format!(
            "{message}: URI: {request_uri}, Status: {status_code}"
        )),
        level: Level::Error,
        exception: vec![exception].into(),
        ..Default::default()
    };

    sentry::capture_event(event);
}

fn log_to_sentry(context: &str, detail: String, request_path: &str, status_code: StatusCode) {
    let breadcrumb = Breadcrumb {
        message: Some(format!("Request at {request_path}")),
        category: Some("request".into()),
        level: Level::Info,
        ..Default::default()
    };

    sentry::add_breadcrumb(breadcrumb);

    let event = Event {
        message: Some(format!("{status_code} - {context}: {detail}")),
        level: Level::Error,
        ..Default::default()
    };

    sentry::capture_event(event);
}

#[cfg(test)]
mod tests {
    use super::*;
    use axum::middleware;
    use axum::{
        Router,
        body::{Body, to_bytes},
        extract::Extension,
        http::{Request as HttpRequest, StatusCode},
        routing::get,
    };
    use rstest::{fixture, rstest};
    use std::sync::Arc;
    use tower::ServiceExt;

    #[derive(Clone)]
    struct HandlerParams {
        status_code: StatusCode,
        body: Option<String>,
    }

    async fn test_handler(Extension(params): Extension<Arc<HandlerParams>>) -> impl IntoResponse {
        let body = params.body.clone().unwrap_or_else(|| "".to_string());
        Response::builder()
            .status(params.status_code)
            .body(Body::from(body))
            .unwrap()
    }

    #[fixture]
    fn request_path() -> &'static str {
        "/test"
    }

    #[fixture]
    fn app() -> Router {
        Router::new()
    }

    #[rstest]
    // Timeout -> bf internal server error user
    #[case(
        StatusCode::REQUEST_TIMEOUT,
        None,
        StatusCode::INTERNAL_SERVER_ERROR,
        Some(BlockfrostError::internal_server_error_user().message)
    )]
    // Method not allowed -> bf bad request
    #[case(
        StatusCode::METHOD_NOT_ALLOWED,
        None,
        StatusCode::BAD_REQUEST,
        Some(BlockfrostError::method_not_allowed().message)
    )]
    // Bad request -> passes through with body preserved
    #[case(
        StatusCode::BAD_REQUEST,
        Some(r#"{"error":"Bad Request","message":"hola hola skola vola","status_code":400}"#),
        StatusCode::BAD_REQUEST,
        Some("hola hola skola vola".to_string())
    )]
    // Not found -> passes through with body preserved
    #[case(
        StatusCode::NOT_FOUND,
        Some(r#"{"error":"Not Found","message":"nic","status_code":404}"#),
        StatusCode::NOT_FOUND,
        Some("nic".to_string())
    )]
    // Success
    #[case(StatusCode::OK, Some("Success"), StatusCode::OK, None)]
    #[tokio::test]
    async fn test_error_middleware(
        #[case] handler_status: StatusCode,
        #[case] handler_body: Option<&'static str>,
        #[case] expected_status: StatusCode,
        #[case] expected_error_message: Option<String>,
        app: Router,
        request_path: &str,
    ) {
        // Prepare
        let handler_params = Arc::new(HandlerParams {
            status_code: handler_status,
            body: handler_body.map(|s| s.to_string()),
        });

        // Build
        let app = app
            .route(
                request_path,
                get(test_handler).layer(Extension(handler_params)),
            )
            .layer(middleware::from_fn(error_middleware));

        // Send a request
        let response = app
            .oneshot(
                HttpRequest::builder()
                    .uri(request_path)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), expected_status);

        let body_bytes = to_bytes(response.into_body(), usize::MAX).await.unwrap();

        if let Some(expected_message) = expected_error_message {
            // debug for test
            // println!("expected_message {}", expected_message);
            // println!("expected_error_message {:?}", expected_error_message);

            // Parse the response as BlockfrostError
            let error: BlockfrostError = serde_json::from_slice(&body_bytes).unwrap();
            assert_eq!(error.message, expected_message);
        } else {
            // Successful response
            let body_str = String::from_utf8(body_bytes.to_vec()).unwrap();
            if let Some(expected_body) = handler_body {
                assert_eq!(body_str, expected_body);
            } else {
                assert_eq!(body_str, "");
            }
        }
    }
}
