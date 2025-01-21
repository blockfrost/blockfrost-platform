use axum::{
    middleware::from_fn,
    routing::{get, post},
    Extension, Router,
};
use tower::ServiceBuilder;
use tower_http::normalize_path::{NormalizePath, NormalizePathLayer};

use crate::{
    api::{metrics::setup_metrics_recorder, root, tx_submit},
    cbor::fallback_decoder::FallbackDecoder,
    cli::Config,
    errors::{AppError, BlockfrostError},
    icebreakers_api::IcebreakersAPI,
    middlewares::{errors::error_middleware, metrics::track_http_metrics},
    node::pool::NodePool,
};

/// Builds and configures the Axum `Router`.
/// Returns `Ok(Router)` on success or an `AppError` if a step fails.
pub async fn build(config: &Config) -> Result<(NormalizePath<Router>, NodePool), AppError> {
    // 1. Set up fallback decoder
    let fallback_decoder = FallbackDecoder::spawn()?;

    fallback_decoder
        .startup_sanity_test()
        .await
        .map_err(AppError::Server)?;

    // 2. Create node pool
    let node_conn_pool = NodePool::new(config, fallback_decoder)?;

    // 3. Set up optional Icebreakers API (solitary mode)
    let icebreakers_api = IcebreakersAPI::new(config).await?;

    // 4. Metrics recorder
    let prometheus_handle = if config.metrics {
        Some(setup_metrics_recorder())
    } else {
        None
    };

    // 5. Figure out prefix
    let api_prefix = if let Some(api) = &icebreakers_api {
        api.api_prefix.clone()
    } else {
        "/".to_string()
    };

    // 6. Routes
    let api_routes = Router::new()
        .route("/", get(root::route))
        .route("/tx/submit", post(tx_submit::route))
        .route("/metrics", get(crate::api::metrics::route))
        .layer(Extension(prometheus_handle))
        .layer(Extension(node_conn_pool.clone()))
        .layer(Extension(icebreakers_api))
        .layer(from_fn(error_middleware))
        .fallback(BlockfrostError::not_found())
        .route_layer(from_fn(track_http_metrics));

    // 7. Nest prefix
    let app = if api_prefix == "/" || api_prefix.is_empty() {
        Router::new().merge(api_routes)
    } else {
        Router::new().nest(&api_prefix, api_routes)
    };

    // 8. Final layers (e.g., trim trailing slash)
    let app = ServiceBuilder::new()
        .layer(NormalizePathLayer::trim_trailing_slash())
        .service(app);

    Ok((app, node_conn_pool))
}
