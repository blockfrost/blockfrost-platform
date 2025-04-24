use crate::{
    api::{
        accounts, addresses, assets, blocks, epochs, governance, health, ledger,
        metrics::setup_metrics_recorder, network, pools, root, scripts, tx, txs, utils,
    },
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
            .route("/accounts/{stake_address}", get(accounts::stake_address::route))
            .route("/accounts/{stake_address}/rewards", get(accounts::stake_address::rewards::route))
            .route("/accounts/{stake_address}/history", get(accounts::stake_address::history::route))
            .route("/accounts/{stake_address}/delegations", get(accounts::stake_address::delegations::route))
            .route("/accounts/{stake_address}/registrations", get(accounts::stake_address::registrations::route))
            .route("/accounts/{stake_address}/withdrawals", get(accounts::stake_address::withdrawals::route))
            .route("/accounts/{stake_address}/mirs", get(accounts::stake_address::mirs::route))
            .route("/accounts/{stake_address}/addresses", get(accounts::stake_address::addresses::route))
            .route("/accounts/{stake_address}/addresses/assets", get(accounts::stake_address::addresses::assets::route))
            .route("/accounts/{stake_address}/addresses/total", get(accounts::stake_address::addresses::total::route))
            .route("/accounts/{stake_address}/utxos", get(accounts::stake_address::utxos::route))

            // addresses
            .route("/addresses/{address}", get(addresses::address::route))
            .route("/addresses/{address}/extended", get(addresses::address::extended::route))
            .route("/addresses/{address}/total", get(addresses::address::total::route))
            .route("/addresses/{address}/utxos", get(addresses::address::utxos::route))
            .route("/addresses/{address}/utxos/{asset}", get(addresses::address::utxos::asset::route))
            .route("/addresses/{address}/txs", get(addresses::address::txs::route))
            .route("/addresses/{address}/transactions", get(addresses::address::transactions::route))

            // assets
            .route("/assets", get(assets::root::route))
            .route("/assets/{asset}", get(assets::asset::root::route))
            .route("/assets/{asset}/history", get(assets::asset::history::route))
            .route("/assets/{asset}/txs", get(assets::asset::txs::route))
            .route("/assets/{asset}/transactions", get(assets::asset::transactions::route))
            .route("/assets/{asset}/addresses", get(assets::asset::addresses::route))
            .route("/assets/policy/{policy_id}", get(assets::policy::policy_id::route))

            // blocks
            .route("blocks/latest", get(blocks::latest::root::route))
            .route("blocks/latest/txs", get(blocks::latest::txs::route))
            .route("blocks/{hash_or_number}/addresses", get(blocks::hash_or_number::addresses::route))
            .route("blocks/{hash_or_number}/next", get(blocks::hash_or_number::next::route))
            .route("blocks/{hash_or_number}/previous", get(blocks::hash_or_number::previous::route))
            .route("blocks/{hash_or_number}/txs", get(blocks::hash_or_number::txs::route))

            // epochs
            .route("epochs/latest", get(epochs::latest::root::route))
            .route("epochs/latest/parameters", get(epochs::latest::parameters::route))
            .route("epochs/{number}", get(epochs::number::root::route))
            .route("epochs/{number}/next", get(epochs::number::next::route))
            .route("epochs/{number}/previous", get(epochs::number::previous::route))
            .route("epochs/{number}/stakes", get(epochs::number::stakes::route))
            .route("epochs/{number}/stakes/{pool_id}", get(epochs::number::stakes::pool_id::route))
            .route("epochs/{number}/blocks", get(epochs::number::blocks::route))
            .route("epochs/{number}/blocks/{pool_id}", get(epochs::number::blocks::pool_id::route))
            .route("epochs/{number}/parameters", get(epochs::number::parameters::route))

            // health
            .route("/health", get(health::root::route))
            .route("/health/clock", get(health::clock::route))

            // ledger
            .route("geensis", get(ledger::genesis::route))

            // governance
            .route("governance/dreps", get(governance::dreps::route))
            .route("governance/dreps/{drep_id}", get(governance::dreps::route))
            .route("governance/dreps/{drep_id}/delegators", get(governance::dreps::route))
            .route("governance/dreps/{drep_id}/metadata", get(governance::dreps::route))
            .route("governance/dreps/{drep_id}/updates", get(governance::dreps::route))
            .route("governance/dreps/{drep_id}/votes", get(governance::dreps::route))
            .route("governance/proposals", get(governance::proposals::route))
            .route("governance/proposals/{tx_hash}/{cert_index}", get(governance::proposals::route))
            .route("governance/proposals/{tx_hash}/{cert_index}/parameters", get(governance::proposals::route))
            .route("governance/proposals/{tx_hash}/{cert_index}/withdrawals", get(governance::proposals::route))
            .route("governance/proposals/{tx_hash}/{cert_index}/votes", get(governance::proposals::route))
            .route("governance/proposals/{tx_hash}/{cert_index}/metadata", get(governance::proposals::route))

            // metadata
            .route("metadata/txs/labels", get(metadata::root::route))
            .route("metadata/txs/labels/{label}", get(metadata::root::route))
            .route("metadata/txs/labels/{label}/cbor", get(metadata::root::route))

            // network
            .route("/network", get(network::root::route))
            .route("/network/eras", get(network::eras::route))

            // pools
            .route("/pools", get(pools::root::route))
            .route("/pools/extended", get(pools::hash_or_number::route))
            .route("/pools/retired", get(pools::hash_or_number::route))
            .route("/pools/retiring", get(pools::hash_or_number::route))
            .route("/pools/{pool_id}", get(pools::hash_or_number::route))
            .route("/pools/{pool_id}/history", get(pools::hash_or_number::route))
            .route("/pools/{pool_id}/metadata", get(pools::hash_or_number::route))
            .route("/pools/{pool_id}/relays", get(pools::hash_or_number::route))
            .route("/pools/{pool_id}/delegators", get(pools::hash_or_number::route))
            .route("/pools/{pool_id}/blocks", get(pools::hash_or_number::route))
            .route("/pools/{pool_id}/updates", get(pools::hash_or_number::route))
            .route("/pools/{pool_id}/votes", get(pools::hash_or_number::route))

            // tx
            .route("/tx/submit", post(tx::submit::route))

            // scripts
            .route("/scripts", post(tx::submit::route))
            .route("/scripts/{script_hash}/json", post(tx::submit::route))
            .route("/scripts/{script_hash}/cbor", post(tx::submit::route))
            .route("/scripts/{script_hash}/redeemers", post(tx::submit::route))
            .route("/scripts/{script_hash}/json", post(tx::submit::route))
            .route("/scripts/datum/{datum_hash}", post(tx::submit::route))
            .route("/scripts/datum/{datum_hash}/cbor", post(tx::submit::route))

            // txs
            .route("/txs/{hash}", get(txs::root::route))
            .route("/txs/{hash}/utxos", get(txs::hash_or_number::route))
            .route("/txs/{hash}/stakes", get(txs::hash_or_number::addresses::route))
            .route("/txs/{hash}/delegations", get(txs::hash_or_number::addresses::route))
            .route("/txs/{hash}/withdrawals", get(txs::hash_or_number::addresses::route))
            .route("/txs/{hash}/mirs", get(txs::hash_or_number::addresses::route))
            .route("/txs/{hash}/pool_updates", get(txs::hash_or_number::addresses::route))
            .route("/txs/{hash}/pool_retires", get(txs::hash_or_number::addresses::route))
            .route("/txs/{hash}/metadata", get(txs::hash_or_number::addresses::route))
            .route("/txs/{hash}/metadata/cbor", get(txs::hash_or_number::addresses::route))
            .route("/txs/{hash}/redeemers", get(txs::hash_or_number::addresses::route))
            .route("/txs/{hash}/required_signers", get(txs::hash_or_number::addresses::route))
            .route("/txs/{hash}/cbor", get(txs::hash_or_number::addresses::route))

            // utils
            .route("/utils", get(utils::root::route));

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
