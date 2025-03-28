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
    // Setting up the metrics recorder needs to be the very first step before
    // doing anything that uses metrics, or the initial data will be lost:
    let metrics = if !config.no_metrics {
        Some(setup_metrics_recorder())
    } else {
        None
    };

    // Export process metrics (memory, CPU time, fds, threads):
    if metrics.is_some() {
        tokio::spawn(async {
            let collector = metrics_process::Collector::default();
            collector.describe();
            loop {
                collector.collect();
                tokio::time::sleep(std::time::Duration::from_secs(5)).await
            }
        });
    }

    // Create node pool
    let node_conn_pool = NodePool::new(&config)?;

    let health_monitor = health_monitor::HealthMonitor::spawn(node_conn_pool.clone()).await;

    // Build a prefix
    let api_prefix = if config.icebreakers_config.is_some() {
        format!("/{}", Uuid::new_v4())
    } else {
        "/".to_string()
    };

    // Set up optional Icebreakers API (solitary option in CLI)
    let icebreakers_api = IcebreakersAPI::new(&config, api_prefix.clone()).await?;

    // API routes that are always under / (and also under the UUID prefix, if we use it)
    let regular_api_routes = {
        let mut rv = Router::new().route("/", get(root::route));
        if metrics.is_some() {
            rv = rv
                .route("/metrics", get(crate::api::metrics::route))
                .route_layer(from_fn(track_http_metrics));
        }
        rv
    };

    // API routes that are *only* under the UUID prefix
    let hidden_api_routes = {
        let mut rv = Router::new().route("/tx/submit", post(tx_submit::route));
        if metrics.is_some() {
            rv = rv.route_layer(from_fn(track_http_metrics));
        }
        rv
    };

    // Nest under the UUID prefix
    let api_routes = if api_prefix == "/" || api_prefix.is_empty() {
        regular_api_routes.merge(hidden_api_routes)
    } else {
        regular_api_routes
            .clone()
            .nest(&api_prefix, regular_api_routes.merge(hidden_api_routes))
    };

    // Add layers
    let app = {
        let mut rv = api_routes
            .layer(Extension(config))
            .layer(Extension(health_monitor))
            .layer(Extension(node_conn_pool.clone()))
            .layer(from_fn(error_middleware))
            .fallback(BlockfrostError::not_found_with_uri);
        if let Some(m) = metrics {
            rv = rv.layer(Extension(m));
        }
        rv
    };

    // Final layers (e.g., trim trailing slash)
    let app = ServiceBuilder::new()
        .layer(NormalizePathLayer::trim_trailing_slash())
        .service(app);

    Ok((app, node_conn_pool, icebreakers_api, api_prefix))
}
