use crate::AppError;
use crate::cli::Args;
use crate::genesis::{GenesisRegistry, genesis};
use clap::ValueEnum;
use pallas_network::facades::NodeClient;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Formatter};
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
    pub fn from_args_with_detector<F>(args: Args, detector: F) -> Result<Self, AppError>
    where
        F: Fn(&str) -> Result<Network, AppError>,
    {
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

        let network = detector(&node_socket_path)?;

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
        })
    }

    pub fn from_args(args: Args) -> Result<Self, AppError> {
        Self::from_args_with_detector(args, detect_network)
    }
}

fn detect_network(socket_path: &str) -> Result<Network, AppError> {
    let magics = genesis().all_magics();

    let network = tokio::task::block_in_place(|| {
        let runtime = tokio::runtime::Runtime::new()
            .map_err(|e| AppError::Server(format!("Runtime creation failed: {}", e)))?;

        for magic in magics {
            let ok = runtime.block_on(async {
                match NodeClient::connect(socket_path, magic).await {
                    Ok(conn) => {
                        conn.abort().await;
                        true
                    },
                    Err(_) => false,
                }
            });

            if ok {
                return Ok(genesis().network_by_magic(magic).clone());
            }
        }

        Err(AppError::Server(format!(
            "Could not detect network from socket path '{}'. Is the Cardano node running and accessible?",
            socket_path
        )))
    })?;

    Ok(network)
}
