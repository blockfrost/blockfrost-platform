use crate::AppError;
use clap::{arg, command, Parser, ValueEnum};
use pallas_network::miniprotocols::{MAINNET_MAGIC, PREPROD_MAGIC, PREVIEW_MAGIC};
use std::env;
use std::fmt::{self, Formatter};
use tracing::Level;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
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

#[derive(Debug, Clone, ValueEnum)]
pub enum Mode {
    Compact,
    Light,
    Full,
}

#[derive(Debug, Clone, ValueEnum)]
pub enum Network {
    Mainnet,
    Preprod,
    Preview,
}

#[derive(Debug, Clone, ValueEnum)]
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
        let server_address = match env::var("SERVER_ADDRESS") {
            Ok(val) => val,
            Err(_) => args.server_address,
        };

        let server_port = match env::var("SERVER_PORT") {
            Ok(val) => val.parse::<u16>().unwrap_or(args.server_port),
            Err(_) => args.server_port,
        };

        let node_socket_path = match env::var("NODE_SOCKET_PATH") {
            Ok(val) => val,
            Err(_) => args.node_socket_path,
        };

        // For the network, parse an env var if present and convert it to the enum.
        // If parsing fails or not set, keep the CLI version.
        let network = match env::var("NETWORK") {
            Ok(val) => match val.to_lowercase().as_str() {
                "mainnet" => Network::Mainnet,
                "preprod" => Network::Preprod,
                "preview" => Network::Preview,
                _ => args.network, // fallback
            },
            Err(_) => args.network,
        };

        let log_level = match env::var("LOG_LEVEL") {
            Ok(val) => match val.to_lowercase().as_str() {
                "debug" => LogLevel::Debug.into(),
                "info" => LogLevel::Info.into(),
                "warn" => LogLevel::Warn.into(),
                "error" => LogLevel::Error.into(),
                "trace" => LogLevel::Trace.into(),
                _ => args.log_level.into(),
            },
            Err(_) => args.log_level.into(),
        };

        let mode = match env::var("MODE") {
            Ok(val) => match val.to_lowercase().as_str() {
                "compact" => Mode::Compact,
                "light" => Mode::Light,
                "full" => Mode::Full,
                _ => args.mode,
            },
            Err(_) => args.mode,
        };

        let metrics = match env::var("METRICS") {
            Ok(val) => val.to_lowercase() == "true",
            Err(_) => args.metrics,
        };

        let icebreakers_config = match (
            args.solitary,
            args.reward_address.clone(),
            args.secret.clone(),
        ) {
            (false, Some(reward_address), Some(secret)) => {
                let reward_address = env::var("REWARD_ADDRESS").unwrap_or(reward_address);
                let secret = env::var("SECRET").unwrap_or(secret);

                Some(IcebreakersConfig {
                    reward_address,
                    secret,
                })
            }
            _ => None,
        };

        let network_magic = Self::get_network_magic(&network);

        Ok(Config {
            max_pool_connections: 10,
            server_address,
            server_port,
            log_level,
            network_magic,
            node_socket_path,
            mode,
            icebreakers_config,
            network,
            metrics,
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
