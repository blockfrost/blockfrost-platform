use axum::Router;
use blockfrost_platform::{
    cli::{Config, LogLevel, Mode, Network},
    server::build,
    AppError, NodePool,
};
use lazy_static::lazy_static;
use std::sync::Once;
use tower_http::normalize_path::NormalizePath;

lazy_static! {
    static ref INIT: Once = Once::new();
}

pub fn initialize_logging() {
    INIT.call_once(|| {
        tracing_subscriber::fmt::init();
    });
}

pub fn test_config() -> Config {
    Config {
        server_address: "0.0.0.0".into(),
        server_port: 8080,
        log_level: LogLevel::Info.into(),
        network_magic: 2,
        mode: Mode::Compact,
        node_socket_path: "/run/cardano-node/node.socket".into(),
        icebreakers_config: None,
        max_pool_connections: 10,
        network: Network::Preview,
    }
}

pub async fn build_app() -> Result<(NormalizePath<Router>, NodePool), AppError> {
    let config = test_config();

    build(&config).await
}
