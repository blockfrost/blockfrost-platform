use crate::{api::root, middlewares::metrics::track_http_metrics, server::state::AppState};
use axum::{Router, middleware::from_fn, routing::get};
use tower_http::normalize_path::NormalizePathLayer;

pub fn get_regular_api_routes(enable_metrics: bool) -> Router<AppState> {
    let mut router = Router::new()
        .route("/", get(root::route))
        .route_layer(NormalizePathLayer::trim_trailing_slash());

    if enable_metrics {
        router = router
            .route("/metrics", get(crate::api::metrics::route))
            .route_layer(from_fn(track_http_metrics));
    }

    router
}
