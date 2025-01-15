use clap::{arg, command, Parser, ValueEnum};
use pallas_network::miniprotocols::{MAINNET_MAGIC, PREPROD_MAGIC, PREVIEW_MAGIC};
use std::{
    env,
    fmt::{self, Formatter},
};
use tracing::Level;

use crate::AppError;

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
    pub testgen_hs_path: String,
}

pub struct IcebreakersConfig {
    pub reward_address: String,
    pub secret: String,
}

impl Config {
    pub fn from_args(args: Args) -> Result<Self, AppError> {
        let testgen_hs_path = env::var("TESTGEN_HS_PATH")?;
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
            testgen_hs_path,
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
