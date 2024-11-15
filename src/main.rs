mod api;
mod background_tasks;
mod cbor;
mod cli;
mod common;
mod errors;
mod icebreakers_api;
mod middlewares;
mod node;

use api::metrics::setup_metrics_recorder;
use api::root;
use api::tx;
use axum::extract::Request;
use axum::middleware::from_fn;
use axum::routing::{get, post};
use axum::Extension;
use axum::{Router, ServiceExt};
use background_tasks::node_health_check_task;
use clap::Parser;
use cli::Args;
use cli::Config;
use errors::AppError;
use errors::BlockfrostError;
use icebreakers_api::IcebreakersAPI;
use middlewares::errors::error_middleware;
use middlewares::metrics::track_http_metrics;
use node::pool::NodePool;
use tower::ServiceBuilder;
use tower_http::normalize_path::NormalizePathLayer;
use tracing::info;
use tracing_subscriber::fmt::format::Format;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    let arguments = Args::parse();
    let config = Config::from_args(arguments)?;

    tracing_subscriber::fmt()
        .with_max_level(config.log_level)
        .event_format(
            Format::default()
                .with_ansi(true)
                .with_level(true)
                .with_target(false)
                .compact(),
        )
        .init();

    let node_conn_pool = NodePool::new(&config)?;
    let icebreakers_api = IcebreakersAPI::new(&config).await?;
    let prometheus_handle = setup_metrics_recorder();

    let api_prefix = if let Some(api) = &icebreakers_api {
        api.read()
            .map_err(|_| {
                AppError::Registration("Failed to acquire read lock on IcebreakersAPI".into())
            })?
            .api_prefix
            .clone()
            .unwrap_or("/".to_string())
    } else {
        "/".to_string()
    };

    let api_routes = Router::new()
        .route("/", get(root::route))
        .route("/tx/submit", post(tx::submit::route))
        .route("/metrics", get(api::metrics::route))
        .layer(Extension(prometheus_handle))
        .layer(Extension(node_conn_pool.clone()))
        .layer(Extension(icebreakers_api))
        .layer(from_fn(error_middleware))
        .fallback(BlockfrostError::not_found())
        .route_layer(from_fn(track_http_metrics));

    let app = Router::new().nest(api_prefix.as_str(), api_routes);
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
