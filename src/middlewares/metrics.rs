use axum::extract::{MatchedPath, Request};
use axum::middleware::Next;
use axum::response::IntoResponse;
use metrics::counter;

pub async fn track_http_metrics(req: Request, next: Next) -> impl IntoResponse {
    let path = if let Some(matched_path) = req.extensions().get::<MatchedPath>() {
        matched_path.as_str().to_owned()
    } else {
        req.uri().path().to_owned()
    };

    let method = req.method().clone();
    let response = next.run(req).await;
    let status = response.status().as_u16().to_string();

    let labels = [
        ("method", method.to_string()),
        ("path", path),
        ("status", status),
    ];

    counter!("http_requests_total", &labels).increment(1);

    response
}
