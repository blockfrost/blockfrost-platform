use axum::extract::{MatchedPath, Request};
use axum::middleware::Next;
use axum::response::IntoResponse;
use metrics::counter;

pub async fn track_http_metrics(req: Request, next: Next) -> impl IntoResponse {
    let route = req
        .extensions()
        .get::<MatchedPath>()
        .map(MatchedPath::as_str)
        .unwrap_or("unmatched")
        .to_owned();
    let method = req.method().to_string();

    let response = next.run(req).await;
    let status_code = response.status().as_u16().to_string();

    let labels = [
        ("method", method),
        ("route", route),
        ("status_code", status_code),
    ];

    counter!("blockfrost_gateway_http_requests_total", &labels).increment(1);

    response
}
