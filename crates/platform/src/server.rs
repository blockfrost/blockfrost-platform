pub mod logging;
pub mod metrics;
pub mod routes;
pub mod state;
use crate::{
    health_monitor, icebreakers::api::IcebreakersAPI, middlewares::errors::error_middleware,
};
use axum::{Extension, Router, middleware::from_fn};
use bf_common::{
    config::{Config, Evaluator},
    errors::{AppError, BlockfrostError},
};
use bf_data_node::client::DataNode;
use bf_node::{chain_config::init_caches, pool::NodePool};
use bf_tx_evaluator::external::ExternalEvaluator;
use metrics::{setup_metrics_recorder, spawn_process_collector};
use routes::{hidden::get_hidden_api_routes, nest_routes, regular::get_regular_api_routes};
use state::{ApiPrefix, AppState};
use std::sync::Arc;
use tower::{Layer, limit::ConcurrencyLimitLayer};
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
    let metrics_handle = if !config.no_metrics {
        let recorder = setup_metrics_recorder();
        spawn_process_collector();

        Some(recorder)
    } else {
        None
    };

    // Create node pool
    let node_conn_pool = NodePool::new(&config)?;

    // Data node
    let data_node = config.data_node.as_ref().map(DataNode::new).transpose()?;

    // Health monitor
    let health_monitor =
        health_monitor::HealthMonitor::spawn(node_conn_pool.clone(), data_node.clone()).await;

    // Build a prefix
    let api_prefix = ApiPrefix(config.icebreakers_config.as_ref().map(|_| Uuid::new_v4()));

    // Set up optional Icebreakers API (solitary option in CLI)
    let icebreakers_api = IcebreakersAPI::new(&config, api_prefix.clone()).await?;

    // Initialize chain configurations
    let chain_config_cache = init_caches(node_conn_pool.clone()).await?;

    let fallback_evaluator = if config.evaluator == Evaluator::External {
        // Initialize the Haskell-based tx evaluator
        Some(ExternalEvaluator::spawn(chain_config_cache).await?)
    } else {
        None
    };

    // API routes that are always under / (and also under the UUID prefix, if we use it)
    let regular_api_routes = get_regular_api_routes(!config.no_metrics);
    let hidden_api_routes = get_hidden_api_routes(!config.no_metrics);

    // Nest under the UUID prefix
    let api_routes = nest_routes(&api_prefix, regular_api_routes, hidden_api_routes);

    let genesis = Arc::new(config.with_custom_genesis()?);

    // Initialize the app state
    let app_state = AppState {
        config: config.clone(),
        genesis,
        data_node,
    };

    // Add layers
    let inner = {
        let mut routes = api_routes
            .with_state(app_state.clone())
            .layer(Extension(health_monitor.clone()))
            .layer(Extension(node_conn_pool.clone()))
            .layer(Extension(fallback_evaluator))
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

    Ok((
        app,
        node_conn_pool,
        health_monitor,
        icebreakers_api,
        api_prefix,
    ))
}
