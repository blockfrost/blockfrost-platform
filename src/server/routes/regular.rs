use crate::{api::root, middlewares::metrics::track_http_metrics, server::state::AppState};
use axum::{Router, middleware::from_fn, routing::get};

pub fn get_regular_api_routes(enable_metrics: bool) -> Router<AppState> {
    let mut router = Router::new().route("/", get(root::route));

    if enable_metrics {
        router = router
            .route("/metrics", get(crate::api::metrics::route))
            .route_layer(from_fn(track_http_metrics));
    }

    router
}
