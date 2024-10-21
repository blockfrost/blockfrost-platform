mod api;
mod cli;
mod common;
mod errors;
mod icebreakers_api;
mod middlewares;
mod node;

use api::root;
use api::tx;
use axum::extract::Request;
use axum::middleware::from_fn;
use axum::routing::{get, post};
use axum::Extension;
use axum::{Router, ServiceExt};
use clap::Parser;
use cli::Args;
use cli::Config;
use errors::AppError;
use errors::BlockfrostError;
use middlewares::errors::error_middleware;
use std::sync::Arc;
use tokio::sync::RwLock;
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

    let node = Arc::new(RwLock::new(
        node::Node::new(&config.node_socket_path, config.network_magic, &config.node_bin_path).await?,
    ));

    let icebreakers_api = Arc::new(RwLock::new(
        icebreakers_api::IcebreakersAPI::new(&config).await?,
    ));

    let app = Router::new()
        .route("/", get(root::route))
        .route("/tx/submit", post(tx::submit::route))
        .layer(Extension(node))
        .layer(Extension(icebreakers_api))
        .layer(from_fn(error_middleware))
        .fallback(BlockfrostError::not_found());

    let app = ServiceBuilder::new()
        .layer(NormalizePathLayer::trim_trailing_slash())
        .service(app);

    let addr = format!("{}:{}", config.server_address, config.server_port);
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    info!("Server is listening on {}", addr);
    info!("Log level {}", config.log_level);
    info!("Mode {}", config.mode);

    axum::serve(listener, ServiceExt::<Request>::into_make_service(app)).await?;

    Ok(())
}
