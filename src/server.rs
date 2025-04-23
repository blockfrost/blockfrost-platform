use crate::{
    api::{blocks, health, metrics::setup_metrics_recorder, network, tx},
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
use tower_http::normalize_path::NormalizePathLayer;
use uuid::Uuid;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApiPrefix(pub Option<Uuid>);

impl std::fmt::Display for ApiPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(uuid) => write!(f, "/{}", uuid),
            None => write!(f, "/"),
        }
    }
}

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
    let api_prefix = ApiPrefix(config.icebreakers_config.as_ref().map(|_| Uuid::new_v4()));

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
        let mut rv = Router::new()
            // accounts
            .route("/accounts", get(accounts::root::route))
            .route("/accounts/{account}/blocks", get(accounts::account::blocks::route))
            .route("/accounts/{account}/txs", get(accounts::account::txs::route))

            // addresses
            .route("addresses/{address}/blocks", get(addresses::address::blocks::route))
            .route("addresses/{address}/txs", get(addresses::address::txs::route))

            // assets
            .route("assets/{asset}/addresses", get(assets::asset::addresses::route))
            .route("assets/{asset}/txs", get(assets::asset::txs::route))

            // blocks
            .route("blocks/latest", get(blocks::latest::root::route))
            .route("blocks/latest/txs", get(blocks::latest::txs::route))
            .route("blocks/{hash_or_number}/addresses", get(blocks::hash_or_number::addresses::route))
            .route("blocks/{hash_or_number}/next", get(blocks::hash_or_number::next::route))
            .route("blocks/{hash_or_number}/previous", get(blocks::hash_or_number::previous::route))
            .route("blocks/{hash_or_number}/txs", get(blocks::hash_or_number::txs::route))

            // epochs
            .route("epochs/latest", get(epochs::latest::root::route))
            .route("epochs/latest/txs", get(epochs::latest::txs::route))
            .route("epochs/{hash_or_number}/addresses", get(epochs::hash_or_number::addresses::route))
            .route("epochs/{hash_or_number}/next", get(epochs::hash_or_number::next::route))
            .route("epochs/{hash_or_number}/previous", get(epochs::hash_or_number::previous::route))
            .route("epochs/{hash_or_number}/txs", get(epochs::hash_or_number::txs::route))

            // health
            .route("/health", get(health::root::route))
            .route("/health/clock", get(health::clock::route))

            // ledger
            .route("geensis", get(ledger::root::route))

            // metadata
            .route("metadata", get(metadata::root::route))
            .route("metadata/latest", get(metadata::latest::root::route))
            .route("metadata/latest/txs", get(metadata::latest::txs::route))
            .route("metadata/{hash_or_number}/addresses", get(metadata::hash_or_number::addresses::route))
            .route("metadata/{hash_or_number}/next", get(metadata::hash_or_number::next::route))
            .route("metadata/{hash_or_number}/previous", get(metadata::hash_or_number::previous::route))
            .route("metadata/{hash_or_number}/txs", get(metadata::hash_or_number::txs::route))

            // network
            .route("/network", get(network::root::route))
            .route("/network/eras", get(network::eras::route))

            // pools
            .route("/pools", get(pools::root::route))
            .route("/pools/{hash_or_number}", get(pools::hash_or_number::route))
            .route("/pools/{hash_or_number}/addresses", get(pools::hash_or_number::addresses::route))
            .route("/pools/{hash_or_number}/next", get(pools::hash_or_number::next::route))
            .route("/pools/{hash_or_number}/previous", get(pools::hash_or_number::previous::route))
            .route("/pools/{hash_or_number}/txs", get(pools::hash_or_number::txs::route))

            // tx
            .route("/tx/submit", post(tx::submit::route));

            // txs
            .route("/txs", get(txs::root::route))
            .route("/txs/{hash_or_number}", get(txs::hash_or_number::route))
            .route("/txs/{hash_or_number}/addresses", get(txs::hash_or_number::addresses::route))

            // utils
            .route("/utils", get(utils::root::route))

        if metrics.is_some() {
            rv = rv.route_layer(from_fn(track_http_metrics));
        }
        rv
    };

    // Nest under the UUID prefix
    let api_routes: Router = if api_prefix.0.is_none() {
        regular_api_routes.merge(hidden_api_routes)
    } else {
        // XXX: using `.nest()` breaks trailing slashes, we need `.nest_service()`:
        regular_api_routes.clone().nest_service(
            &api_prefix.to_string(),
            regular_api_routes.merge(hidden_api_routes),
        )
    };

    // Add layers
    let app = {
        let mut rv = api_routes
            .layer(Extension(config))
            .layer(Extension(health_monitor.clone()))
            .layer(Extension(node_conn_pool.clone()))
            .layer(from_fn(error_middleware))
            .fallback(BlockfrostError::not_found_with_uri);
        if let Some(m) = metrics {
            rv = rv.layer(Extension(m));
        }
        rv
    };

    // Final layers (e.g., trim trailing slash)
    let app = app.layer(NormalizePathLayer::trim_trailing_slash());

    Ok((
        app,
        node_conn_pool,
        health_monitor,
        icebreakers_api,
        api_prefix,
    ))
}
