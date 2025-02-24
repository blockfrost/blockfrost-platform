use axum::Router;
use blockfrost::{BlockFrostSettings, BlockfrostAPI};
use blockfrost_platform::{
    cli::{Config, LogLevel, Mode, Network},
    icebreakers_api::IcebreakersAPI,
    server::build,
    AppError, NodePool,
};
use std::{
    env,
    sync::{Arc, LazyLock},
};
use tower_http::normalize_path::NormalizePath;

static INIT_LOGGING: LazyLock<()> = LazyLock::new(|| {
    tracing_subscriber::fmt::init();
});

pub fn initialize_logging() {
    let _ = INIT_LOGGING;
}

pub fn get_blockfrost_client() -> BlockfrostAPI {
    let settings = BlockFrostSettings::default();

    BlockfrostAPI::new("previewTjUg7ty9Har2JdaRYlzsGs7Wsy6wp8G6", settings)
}

pub fn test_config() -> Arc<Config> {
    dotenvy::dotenv().ok();

    let node_socket_path_env =
        env::var("NODE_SOCKET_PATH").unwrap_or_else(|_| "/run/cardano-node/node.socket".into());

    let config = Config {
        server_address: "0.0.0.0".into(),
        server_port: 8080,
        log_level: LogLevel::Info.into(),
        network_magic: 2,
        mode: Mode::Compact,
        node_socket_path: node_socket_path_env,
        icebreakers_config: None,
        max_pool_connections: 10,
        network: Network::Preview,
        no_metrics: false,
    };

    Arc::new(config)
}

pub async fn build_app() -> Result<
    (
        NormalizePath<Router>,
        NodePool,
        Option<Arc<IcebreakersAPI>>,
        String,
    ),
    AppError,
> {
    let config = test_config();

    build(config).await
}
