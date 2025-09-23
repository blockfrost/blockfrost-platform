use axum::Router;
use blockfrost::{BlockFrostSettings, BlockfrostAPI};
use blockfrost_platform::{
    AppError, health_monitor,
    icebreakers::api::IcebreakersAPI,
    server::{build, state::ApiPrefix},
};
use common::{
    config::{Config, DataSources, DolosConfig, IcebreakersConfig, Mode},
    types::{LogLevel, Network},
};
use node::pool::NodePool;
use std::{
    env,
    path::PathBuf,
    sync::{Arc, LazyLock},
    time::Duration,
};

static INIT_LOGGING: LazyLock<()> = LazyLock::new(|| {
    tracing_subscriber::fmt::init();
});

pub fn initialize_logging() {
    let _ = INIT_LOGGING;
}

pub fn get_blockfrost_client() -> BlockfrostAPI {
    let settings = BlockFrostSettings::default();

    BlockfrostAPI::new("previewy2pbyga8FifUwJSverBCwhESegV6I7gT", settings)
}

#[derive(Default)]
pub struct TestConfigOverrides {
    pub server_port: Option<u16>,
    pub log_level: Option<LogLevel>,
    pub mode: Option<Mode>,
    pub node_socket_path: Option<String>,
    pub icebreakers_config: Option<IcebreakersConfig>,
    pub max_pool_connections: Option<usize>,
    pub network: Option<Network>,
    pub no_metrics: Option<bool>,
    pub custom_genesis_config: Option<PathBuf>,
    pub dolos: Option<DolosConfig>,
}

pub fn test_config(overrides: TestConfigOverrides) -> Arc<Config> {
    dotenvy::dotenv().ok();

    let node_socket_path_env = env::var("CARDANO_NODE_SOCKET_PATH")
        .unwrap_or_else(|_| "/run/cardano-node/node.socket".into());

    let config = Config {
        server_address: "0.0.0.0".parse().unwrap(),
        server_port: overrides.server_port.unwrap_or(3000),
        log_level: overrides.log_level.unwrap_or(LogLevel::Info).into(),
        mode: overrides.mode.unwrap_or(Mode::Compact),
        node_socket_path: overrides.node_socket_path.unwrap_or(node_socket_path_env),
        icebreakers_config: overrides.icebreakers_config,
        max_pool_connections: overrides.max_pool_connections.unwrap_or(10),
        network: overrides.network.unwrap_or(Network::Preview),
        no_metrics: overrides.no_metrics.unwrap_or(false),
        custom_genesis_config: overrides.custom_genesis_config,
        data_sources: DataSources {
            dolos: overrides.dolos,
        },
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
    let config = test_config(Default::default());

    build(config).await
}

pub async fn build_app_with_dolos() -> Result<
    (
        Router,
        NodePool,
        health_monitor::HealthMonitor,
        Option<Arc<IcebreakersAPI>>,
        ApiPrefix,
    ),
    AppError,
> {
    let dolos_config = DolosConfig {
        endpoint: Some("http://localhost:3010".to_string()),
        request_timeout: Duration::from_secs(30),
    };

    let config = test_config(TestConfigOverrides {
        dolos: Some(dolos_config),
        ..Default::default()
    });

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
    let icebreakers_config = IcebreakersConfig {
        secret: "kka0pnx9zqdvh9wl96nsg6sje0f5".to_string(),
        reward_address: "addr_test1qrwlr6uuu2s4v850z45ezjrtj7rnld5kjxgvhjvamjecze3pmjcr2aq4yc35znkn2nfd3agwxy8n7tnaze7tyrjh2snspw9f3g".to_string(),
    };

    let config = test_config(TestConfigOverrides {
        icebreakers_config: Some(icebreakers_config),
        ..Default::default()
    });

    build(config).await
}
