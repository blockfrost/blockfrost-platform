pub mod asserts;
pub mod tx_builder;

use axum::Router;
use bf_common::{
    config::{Config, DataSources, IcebreakersConfig, Mode},
    types::{LogLevel, Network},
};
use bf_node::pool::NodePool;
use blockfrost::{BlockFrostSettings, BlockfrostAPI};
use blockfrost_platform::{
    AppError, health_monitor,
    icebreakers::api::IcebreakersAPI,
    server::{build, state::ApiPrefix},
};
use std::{
    env,
    sync::{Arc, LazyLock},
};

static INIT_LOGGING: LazyLock<()> = LazyLock::new(|| {
    tracing_subscriber::fmt::init();
});

pub fn initialize_logging() {
    let _ = INIT_LOGGING;
}

pub async fn initialize_app() -> Router {
    initialize_logging();
    let (app, _, _, _, _) = build_app().await.expect("Failed to build the application");
    app
}

pub fn get_blockfrost_client() -> BlockfrostAPI {
    let settings = BlockFrostSettings::default();

    BlockfrostAPI::new("previewy2pbyga8FifUwJSverBCwhESegV6I7gT", settings)
}

pub fn test_config(icebreakers_config: Option<IcebreakersConfig>) -> Arc<Config> {
    dotenvy::dotenv().ok();

    let node_socket_path_env = env::var("CARDANO_NODE_SOCKET_PATH")
        .unwrap_or_else(|_| "/run/cardano-node/node.socket".into());

    let config = Config {
        server_address: "0.0.0.0".parse().unwrap(),
        server_port: 3000,
        log_level: LogLevel::Info.into(),
        mode: Mode::Compact,
        node_socket_path: node_socket_path_env,
        icebreakers_config,
        max_pool_connections: 10,
        network: Network::Preview,
        no_metrics: false,
        custom_genesis_config: None,
        data_sources: DataSources { dolos: None },
    };

    Arc::new(config)
}

pub async fn build_app() -> Result<
    (
        Router,
        NodePool,
        health_monitor::HealthMonitor,
        Option<Arc<IcebreakersAPI>>,
        ApiPrefix,
    ),
    AppError,
> {
    let config = test_config(None);

    build(config).await
}

pub async fn build_app_non_solitary() -> Result<
    (
        Router,
        NodePool,
        health_monitor::HealthMonitor,
        Option<Arc<IcebreakersAPI>>,
        ApiPrefix,
    ),
    AppError,
> {
    // Dev secrets for testing
    let icebreakers_config = IcebreakersConfig {
        secret: "kka0pnx9zqdvh9wl96nsg6sje0f5".to_string(),
        reward_address: "addr_test1qrwlr6uuu2s4v850z45ezjrtj7rnld5kjxgvhjvamjecze3pmjcr2aq4yc35znkn2nfd3agwxy8n7tnaze7tyrjh2snspw9f3g".to_string(),
    };
    let config = test_config(Some(icebreakers_config));

    build(config).await
}
