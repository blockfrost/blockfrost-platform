use crate::AppError;
use crate::cli::Args;
use crate::genesis::{GenesisRegistry, genesis};
use blockfrost_openapi::models::genesis_content::GenesisContent;
use clap::ValueEnum;
use futures::FutureExt; // for `.boxed()`
use futures::future::BoxFuture;
use pallas_network::facades::NodeClient;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Formatter};
use std::fs;
use std::path::PathBuf;
use tracing::Level;

#[derive(Clone, Debug)]
pub struct Config {
    pub server_address: std::net::IpAddr,
    pub server_port: u16,
    pub log_level: Level,
    pub node_socket_path: String,
    pub mode: Mode,
    pub icebreakers_config: Option<IcebreakersConfig>,
    pub max_pool_connections: usize,
    pub no_metrics: bool,
    pub network: Network,
    pub custom_genesis_config: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub struct IcebreakersConfig {
    pub reward_address: String,
    pub secret: String,
}

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Compact,
    Light,
    Full,
}

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Mode::Compact => write!(f, "compact"),
            Mode::Light => write!(f, "light"),
            Mode::Full => write!(f, "full"),
        }
    }
}

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Preprod,
    Preview,
    Custom,
}

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Trace,
}

// Implement conversion from LogLevel enum to tracing::Level
impl From<LogLevel> for Level {
    fn from(log_level: LogLevel) -> Self {
        match log_level {
            LogLevel::Debug => Level::DEBUG,
            LogLevel::Info => Level::INFO,
            LogLevel::Warn => Level::WARN,
            LogLevel::Error => Level::ERROR,
            LogLevel::Trace => Level::TRACE,
        }
    }
}

impl Config {
    pub async fn from_args_with_detector(
        args: Args,
        detector: impl for<'a> Fn(&'a str) -> BoxFuture<'a, Result<Network, AppError>>,
    ) -> Result<Self, AppError> {
        let node_socket_path = args
            .node_socket_path
            .ok_or(AppError::Server("--node-socket-path must be set".into()))?;

        let icebreakers_config = if !args.solitary {
            let reward_address = args
                .reward_address
                .ok_or(AppError::Server("--reward-address must be set".into()))?;

            let secret = args
                .secret
                .ok_or(AppError::Server("--secret must be set".into()))?;

            Some(IcebreakersConfig {
                reward_address,
                secret,
            })
        } else {
            if args.reward_address.is_some() || args.secret.is_some() {
                return Err(AppError::Server(
                    "Cannot set --reward-address or --secret in solitary mode (--solitary)".into(),
                ));
            }
            None
        };

        let network = detector(&node_socket_path).await?;

        Ok(Config {
            server_address: args.server_address,
            server_port: args.server_port,
            log_level: args.log_level.into(),
            node_socket_path,
            mode: args.mode,
            icebreakers_config,
            max_pool_connections: 10,
            no_metrics: args.no_metrics,
            network,
            custom_genesis_config: args.custom_genesis_config,
        })
    }

    pub async fn from_args(args: Args) -> Result<Self, AppError> {
        Self::from_args_with_detector(args, |s| detect_network(s).boxed()).await
    }

    /// Build the full genesis registry, overriding or prepending
    /// a user-supplied file if `custom_genesis_config` is Some.
    pub fn with_custom_genesis(&self) -> Result<Vec<(Network, GenesisContent)>, AppError> {
        let mut registry = genesis();

        // if user pointed us at a file, load & insert it
        if let Some(path) = &self.custom_genesis_config {
            let data = fs::read_to_string(path).map_err(|e| {
                AppError::Server(format!(
                    "Failed to read custom genesis file {}: {}",
                    path.display(),
                    e
                ))
            })?;

            // try JSON and TOML
            let custom: GenesisContent = serde_json::from_str(&data)
                .or_else(|_| toml::from_str(&data))
                .map_err(|e| {
                    AppError::Server(format!(
                        "Failed to parse custom genesis file {}: {}",
                        path.display(),
                        e
                    ))
                })?;

            // prepend or replace the entry for custom network
            registry.add(Network::Custom, custom);
        }

        Ok(registry)
    }
}

async fn detect_network(socket_path: &str) -> Result<Network, AppError> {
    let all_magics = genesis().all_magics();

    for magic in all_magics {
        let ok = match NodeClient::connect(&socket_path, magic).await {
            Ok(conn) => {
                conn.abort().await;
                true
            },
            Err(_) => false,
        };

        if ok {
            return Ok(genesis().network_by_magic(magic).clone());
        }
    }

    Err(AppError::Server(format!(
        "Could not detect network from '{socket_path}' is the node running?"
    )))
}
