use axum::{
    extract::Request,
    middleware::from_fn,
    routing::{get, post},
    Extension, Router, ServiceExt,
};
use blockfrost_platform::{
    api::{self, metrics::setup_metrics_recorder, root, tx_submit},
    background_tasks::node_health_check_task,
    cbor::fallback_decoder::FallbackDecoder,
    cli::{Args, Config},
    errors::{AppError, BlockfrostError},
    icebreakers_api::IcebreakersAPI,
    logging::setup_tracing,
    middlewares::{errors::error_middleware, metrics::track_http_metrics},
    node::pool::NodePool,
};
use clap::Parser;
use tower::ServiceBuilder;
use tower_http::normalize_path::NormalizePathLayer;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let arguments = Args::parse();
    let config = Config::from_args(arguments);

    // Setup logging
    setup_tracing(&config);

    // Set up FallbackDecoder
    let fallback_decoder = FallbackDecoder::spawn();

    fallback_decoder
        .startup_sanity_test()
        .await
        .map_err(AppError::Server)?;

    let node_conn_pool = NodePool::new(&config, fallback_decoder)?;
    let icebreakers_api = IcebreakersAPI::new(&config).await?;
    let prometheus_handle = setup_metrics_recorder();

    // Get the API prefix from the Icebreakers API
    let api_prefix = if let Some(api) = &icebreakers_api {
        api.api_prefix.clone()
    } else {
        "/".to_string()
    };

    let api_routes = Router::new()
        .route("/", get(root::route))
        .route("/tx/submit", post(tx_submit::route))
        .route("/metrics", get(api::metrics::route))
        .layer(Extension(prometheus_handle))
        .layer(Extension(node_conn_pool.clone()))
        .layer(Extension(icebreakers_api))
        .layer(from_fn(error_middleware))
        .fallback(BlockfrostError::not_found())
        .route_layer(from_fn(track_http_metrics));

    // Decide whether to nest routes based on api_prefix
    let app = if api_prefix == "/" || api_prefix.is_empty() {
        Router::new().merge(api_routes)
    } else {
        Router::new().nest(&api_prefix, api_routes)
    };

    let app = ServiceBuilder::new()
        .layer(NormalizePathLayer::trim_trailing_slash())
        .service(app);

    let address = format!("{}:{}", config.server_address, config.server_port);
    let listener = tokio::net::TcpListener::bind(&address).await?;

    info!(
        "Server is listening on {}",
        format!("http://{}{}/", address, api_prefix)
    );
    info!("Log level {}", config.log_level);
    info!("Mode {}", config.mode);

    tokio::spawn(node_health_check_task(node_conn_pool));

    axum::serve(listener, ServiceExt::<Request>::into_make_service(app)).await?;

    Ok(())
}

// This is a workaround for the malloc performance issues under heavy multi-threaded load for builds targetting musl, i.e. Alpine Linux
#[cfg(target_env = "musl")]
#[global_allocator]
static GLOBAL: jemalloc::Jemalloc = jemalloc::Jemalloc;
