pub mod logging;
pub mod metrics;
pub mod routes;
pub mod state;

use crate::{
    config::Config,
    errors::{AppError, BlockfrostError},
    health_monitor,
    icebreakers_api::IcebreakersAPI,
    middlewares::errors::error_middleware,
    node::pool::NodePool,
};
use axum::{Extension, Router, middleware::from_fn};
use metrics::{init_metrics, spawn_process_collector_if};
use routes::{hidden::get_hidden_api_routes, nest_routes, regular::get_regular_api_routes};
use state::{ApiPrefix, AppState};
use std::sync::Arc;
use tower_http::normalize_path::NormalizePathLayer;
use uuid::Uuid;

/// Builds and configures the Axum `Router`.
/// Returns `Ok(Router)` on success or an `AppError` if a step fails.
pub async fn build(
    config: Arc<Config>,
) -> Result<
    (
        Router,
        NodePool,
        health_monitor::HealthMonitor,
        Option<Arc<IcebreakersAPI>>,
        ApiPrefix,
    ),
    AppError,
> {
    // Setting up the metrics recorder needs to be the very first step before
    // doing anything that uses metrics, or the initial data will be lost:
    let metrics = init_metrics(config.metrics);
    spawn_process_collector_if(config.metrics);

    // Create node pool
    let node_conn_pool = NodePool::new(&config)?;

    // Health monitor
    let health_monitor = health_monitor::HealthMonitor::spawn(node_conn_pool.clone()).await;

    // Build a prefix
    let api_prefix = ApiPrefix(config.icebreakers_config.as_ref().map(|_| Uuid::new_v4()));

    // Set up optional Icebreakers API (solitary option in CLI)
    let icebreakers_api = IcebreakersAPI::new(&config, api_prefix.clone()).await?;

    // API routes that are always under / (and also under the UUID prefix, if we use it)
    let regular_api_routes = get_regular_api_routes(config.metrics);
    let hidden_api_routes = get_hidden_api_routes(config.metrics);

    // Nest under the UUID prefix
    let api_routes = nest_routes(&api_prefix, regular_api_routes, hidden_api_routes);

    let genesis = Arc::new(config.with_custom_genesis()?);
    let app_state = AppState { config, genesis };

    // Add layers
    let app = {
        let mut rv = api_routes
            .route_layer(NormalizePathLayer::trim_trailing_slash())
            .with_state(app_state.clone())
            .layer(Extension(health_monitor.clone()))
            .layer(Extension(node_conn_pool.clone()))
            .layer(from_fn(error_middleware))
            .fallback(BlockfrostError::not_found_with_uri);
        if let Some(m) = metrics {
            rv = rv.layer(Extension(m));
        }
        rv
    };

    Ok((
        app,
        node_conn_pool,
        health_monitor,
        icebreakers_api,
        api_prefix,
    ))
}
