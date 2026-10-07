pub mod metrics;
pub mod routes;
pub mod state;
use crate::{
    config::Config, genesis::GenesisRegistry, health_monitor, icebreakers::api::IcebreakersAPI,
    middlewares::errors::error_middleware,
};
use axum::{Extension, Router, middleware::from_fn};
use bf_common::errors::{AppError, BlockfrostError};
use bf_data_node::client::DataNode;
use bf_node::pool::NodePool;
use metrics::{setup_metrics_recorder, spawn_process_collector};
use routes::get_api_routes;
use state::AppState;
use std::sync::Arc;
use tower::{Layer, limit::ConcurrencyLimitLayer};
use tower_http::normalize_path::NormalizePathLayer;

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
    ),
    AppError,
> {
    // Setting up the metrics recorder needs to be the very first step before
    // doing anything that uses metrics, or the initial data will be lost:
    let metrics_handle = if !config.no_metrics {
        let recorder = setup_metrics_recorder();
        spawn_process_collector();

        Some(recorder)
    } else {
        None
    };

    // Create node pool
    let node_conn_pool = {
        let network_magic = config.genesis.by_network(&config.network).network_magic as u64;

        NodePool::new(
            network_magic,
            config.node_socket_path.to_string(),
            config.max_pool_connections,
        )?
    };

    // Data node
    let data_node = config
        .data_node
        .as_ref()
        .map(|dn| DataNode::new(&dn.endpoint, dn.request_timeout))
        .transpose()?;

    // Health monitor
    let health_monitor =
        health_monitor::HealthMonitor::spawn(node_conn_pool.clone(), data_node.clone()).await;

    // Set up optional Icebreakers API (solitary option in CLI)
    let icebreakers_api = IcebreakersAPI::new(&config).await?;

    // API routes
    let api_routes = get_api_routes(!config.no_metrics);

    // Initialize the app state
    let app_state = AppState {
        config: config.clone(),
        data_node,
    };

    // Add layers
    let inner = {
        let mut routes = api_routes
            .with_state(app_state.clone())
            .layer(Extension(health_monitor.clone()))
            .layer(Extension(node_conn_pool.clone()))
            .layer(from_fn(error_middleware))
            .fallback(BlockfrostError::not_found());

        if let Some(prom_handler) = metrics_handle {
            routes = routes.layer(Extension(prom_handler));
        }

        routes
    };

    let inner = NormalizePathLayer::trim_trailing_slash().layer(inner);
    let app = Router::new()
        .fallback_service(inner)
        .layer(ConcurrencyLimitLayer::new(config.server_concurrency_limit));

    Ok((app, node_conn_pool, health_monitor, icebreakers_api))
}
