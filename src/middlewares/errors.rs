use crate::BlockfrostError;
use axum::{
    body::{to_bytes, Bytes},
    extract::Request,
    http::StatusCode,
    middleware::Next,
    response::{IntoResponse, Response},
};
use sentry::{
    protocol::{Event, Exception},
    Breadcrumb, Level,
};
use serde_json;
use std::convert::Infallible;

pub async fn error_middleware(request: Request, next: Next) -> Result<Response, Infallible> {
    let request_path = request.uri().path().to_string();
    let response = next.run(request).await;
    let status_code = response.status();

    // Transform timeout to internal server error for user
    // 504 Gateway Timeout
    if response.status() == StatusCode::REQUEST_TIMEOUT {
        return Ok(BlockfrostError::internal_server_error_user().into_response());
    }

    // Transform our custom METHOD_NOT_ALLOWED err
    // to 405 status code
    if response.status() == StatusCode::METHOD_NOT_ALLOWED {
        return Ok(BlockfrostError::method_not_allowed().into_response());
    }

    println!("response:{:?}", response.body());

    // Transform server errors to internal server error for user
    if response.status().is_server_error() {
        handle_server_error(response, &request_path, status_code).await
    } else {
        Ok(response)
    }
}

async fn handle_server_error(
    response: Response,
    request_path: &str,
    status_code: StatusCode,
) -> Result<Response, Infallible> {
    let body = response.into_body();

    match to_bytes(body, usize::MAX).await {
        Ok(bytes) => parse_and_log_error(bytes, request_path, status_code).await,
        Err(e) => {
            log_and_capture_error("Failed to read body", e, request_path, status_code);
        }
    }

    Ok(BlockfrostError::internal_server_error_user().into_response())
}

async fn parse_and_log_error(bytes: Bytes, request_path: &str, status_code: StatusCode) {
    match serde_json::from_slice::<BlockfrostError>(&bytes) {
        Ok(error_info) => {
            tracing::error!(
                "{status_code} in `{}` message: `{}`",
                request_path,
                error_info.message
            );
            log_to_sentry("|", format!("{:?}", error_info), request_path, status_code)
        }
        Err(e) => {
            println!("Failed to parse body as JSON: {:?}", e);
            log_to_sentry(
                "JSON Parse Error",
                format!("{:?}", e),
                request_path,
                status_code,
            );
            let body_str = String::from_utf8_lossy(&bytes);
            println!("Raw Body: {}", body_str);
        }
    }
}

fn log_and_capture_error(
    message: &str,
    error: impl std::fmt::Debug,
    request_path: &str,
    status_code: StatusCode,
) {
    tracing::error!(
        "{}: {:?}, Path: {}, Status: {}",
        message,
        error,
        request_path,
        status_code,
    );

    let exception = Exception {
        ty: "ServerError".to_string(),
        value: Some(format!("{:?}", error)),
        ..Default::default()
    };

    let event = Event {
        message: Some(format!(
            "{}: Path: {}, Status: {}",
            message, request_path, status_code
        )),
        level: Level::Error,
        exception: vec![exception].into(),
        ..Default::default()
    };

    sentry::capture_event(event);
}

fn log_to_sentry(context: &str, detail: String, request_path: &str, status_code: StatusCode) {
    let breadcrumb = Breadcrumb {
        message: Some(format!("Request at {}", request_path)),
        category: Some("request".into()),
        level: Level::Info,
        ..Default::default()
    };

    sentry::add_breadcrumb(breadcrumb);

    let event = Event {
        message: Some(format!("{} - {}: {}", status_code, context, detail)),
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
        body::{to_bytes, Body},
        extract::Extension,
        http::{Request as HttpRequest, StatusCode},
        routing::get,
        Router,
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
