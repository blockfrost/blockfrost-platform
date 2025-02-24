use crate::{
    api::{metrics::setup_metrics_recorder, root, tx_submit},
    cli::Config,
    errors::{AppError, BlockfrostError},
    health_monitor,
    icebreakers_api::IcebreakersAPI,
    middlewares::{errors::error_middleware, metrics::track_http_metrics},
    node::pool::NodePool,
};
use axum::{
    Extension, Router,
    middleware::from_fn,
    routing::{get, post},
};
use std::sync::Arc;
use tower::ServiceBuilder;
use tower_http::normalize_path::{NormalizePath, NormalizePathLayer};
use uuid::Uuid;

/// Builds and configures the Axum `Router`.
/// Returns `Ok(Router)` on success or an `AppError` if a step fails.
pub async fn build(
    config: Arc<Config>,
) -> Result<
    (
        NormalizePath<Router>,
        NodePool,
        Option<Arc<IcebreakersAPI>>,
        String,
    ),
    AppError,
> {
    // Create node pool
    let node_conn_pool = NodePool::new(&config)?;

    let health_monitor = health_monitor::spawn(node_conn_pool.clone()).await;

    // Build a prefix
    let api_prefix = if config.icebreakers_config.is_some() {
        format!("/{}", Uuid::new_v4())
    } else {
        "/".to_string()
    };

    // Set up optional Icebreakers API (solitary option in CLI)
    let icebreakers_api = IcebreakersAPI::new(&config, api_prefix.clone()).await?;

    // Router
    let mut api_routes = Router::new();

    // Add routes
    api_routes = api_routes
        .route("/", get(root::route))
        .route("/tx/submit", post(tx_submit::route));

    let metrics_enabled = !config.no_metrics;

    if metrics_enabled {
        api_routes = api_routes.route("/metrics", get(crate::api::metrics::route));
    }

    // Add layers
    api_routes = api_routes
        .layer(Extension(config))
        .layer(Extension(health_monitor))
        .layer(Extension(node_conn_pool.clone()))
        .layer(from_fn(error_middleware))
        .fallback(BlockfrostError::not_found());

    if metrics_enabled {
        api_routes = api_routes
            .route_layer(from_fn(track_http_metrics))
            .layer(Extension(setup_metrics_recorder()));
    }

    // Nest prefix
    let app = if api_prefix == "/" || api_prefix.is_empty() {
        api_routes
    } else {
        Router::new().nest(&api_prefix, api_routes)
    };

    // Final layers (e.g., trim trailing slash)
    let app = ServiceBuilder::new()
        .layer(NormalizePathLayer::trim_trailing_slash())
        .service(app);

    Ok((app, node_conn_pool, icebreakers_api, api_prefix))
}
