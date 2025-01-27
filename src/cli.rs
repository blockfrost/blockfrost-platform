use crate::AppError;
use clap::CommandFactory;
use clap::{arg, command, Parser, ValueEnum};
use pallas_network::miniprotocols::{MAINNET_MAGIC, PREPROD_MAGIC, PREVIEW_MAGIC};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Formatter};
use tracing::Level;
use twelf::{config, Layer};

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
#[config]
pub struct Args {
    #[arg(long, default_value = "0.0.0.0")]
    server_address: String,

    #[arg(long, default_value = "3000")]
    server_port: u16,

    #[arg(long, required = true)]
    network: Network,

    #[arg(long, default_value = "info")]
    log_level: LogLevel,

    #[arg(long, required = true)]
    node_socket_path: String,

    #[arg(long, default_value = "compact")]
    mode: Mode,

    /// Whether to run in solitary mode, without registering with the Icebreakers API
    #[arg(long)]
    solitary: bool,

    #[arg(
        long,
        required_unless_present("solitary"),
        conflicts_with("solitary"),
        requires("reward_address")
    )]
    secret: Option<String>,

    #[arg(
        long,
        required_unless_present("solitary"),
        conflicts_with("solitary"),
        requires("secret")
    )]
    reward_address: Option<String>,

    #[arg(long, default_value = "true", required = false)]
    metrics: bool,
}

impl Args {
    pub fn init() -> Result<Config, AppError> {
        let matches = Self::command().get_matches();
        let arguments = Self::with_layers(&[
            Layer::Clap(matches),
            Layer::Env(Some(String::from("BLOCKFROST_"))),
        ])
        .map_err(|it| AppError::Server(it.to_string()))?;

        Config::from_args(arguments)
    }
}

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Compact,
    Light,
    Full,
}

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Network {
    Mainnet,
    Preprod,
    Preview,
}

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Trace,
}

#[derive(Clone)]
pub struct Config {
    pub server_address: String,
    pub server_port: u16,
    pub log_level: Level,
    pub network_magic: u64,
    pub node_socket_path: String,
    pub mode: Mode,
    pub icebreakers_config: Option<IcebreakersConfig>,
    pub max_pool_connections: usize,
    pub network: Network,
    pub metrics: bool,
}

#[derive(Clone)]
pub struct IcebreakersConfig {
    pub reward_address: String,
    pub secret: String,
}

impl Config {
    pub fn from_args(args: Args) -> Result<Self, AppError> {
        let network_magic = Self::get_network_magic(&args.network);
        let icebreakers_config = match (args.solitary, args.reward_address, args.secret) {
            (false, Some(reward_address), Some(secret)) => Some(IcebreakersConfig {
                reward_address,
                secret,
            }),
            _ => None,
        };

        Ok(Config {
            server_address: args.server_address,
            server_port: args.server_port,
            log_level: args.log_level.into(),
            network_magic,
            node_socket_path: args.node_socket_path,
            mode: args.mode,
            icebreakers_config,
            max_pool_connections: 10,
            network: args.network,
            metrics: args.metrics,
        })
    }

    fn get_network_magic(network: &Network) -> u64 {
        match network {
            Network::Mainnet => MAINNET_MAGIC,
            Network::Preprod => PREPROD_MAGIC,
            Network::Preview => PREVIEW_MAGIC,
        }
    }
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

impl std::fmt::Display for Mode {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        match self {
            Mode::Compact => write!(f, "compact"),
            Mode::Light => write!(f, "light"),
            Mode::Full => write!(f, "full"),
        }
    }
}
