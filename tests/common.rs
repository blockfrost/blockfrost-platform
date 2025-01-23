use axum::Router;
use blockfrost_platform::{
    cli::{Config, LogLevel, Mode, Network},
    server::build,
    AppError, NodePool,
};
// use pretty_assertions::assert_eq;;
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
        metrics: false,
    };

    Arc::new(config)
}

pub async fn build_app() -> Result<(NormalizePath<Router>, NodePool), AppError> {
    let config = test_config();

    build(config).await
}

// fn prettify_json(s: &[u8]) -> String {
//     let json_value: Value = from_slice(s).expect("Invalid JSON data");

//     serde_json::to_string_pretty(&json_value).expect("Failed to serialize JSON")
// }

// pub fn _compare_pretty_jsons(s1: Bytes, s2: Bytes) {
//     assert_eq!(
//         prettify_json(&s1),
//         prettify_json(&s2),
//         "JSON strings are not equal"
//     );
// }
