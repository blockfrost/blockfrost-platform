use std::fmt::{self, Formatter};

use crate::errors::AppError;
use clap::{arg, command, Parser, ValueEnum};
use pallas_network::miniprotocols::{MAINNET_MAGIC, PREPROD_MAGIC, PREVIEW_MAGIC, SANCHONET_MAGIC};
use tracing::Level;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short = 'a', long, default_value = "0.0.0.0")]
    server_address: String,

    #[arg(short = 'p', long, default_value = "3000")]
    server_port: u16,

    #[arg(short = 'n', long, required = true)]
    network: Network,

    #[arg(short = 'l', long, default_value = "info")]
    log_level: LogLevel,

    #[arg(short = 'd', long, required = true)]
    node_socket_path: String,

    #[arg(short = 'b', long, required = true)]
    node_bin_path: String,

    #[arg(short = 'm', long, default_value = "compact")]
    mode: Mode,

    #[arg(short = 'e', long, required = true)]
    secret: String,

    #[arg(short = 'r', long, required = true)]
    reward_address: String,
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
    Sanchonet,
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
    pub reward_address: String,
    pub node_socket_path: String,
    pub node_bin_path: String,
    pub secret: String,
    pub mode: Mode,
}

impl Config {
    pub fn from_args(args: Args) -> Result<Self, AppError> {
        let network_magic = Self::get_network_magic(args.network)?;

        Ok(Config {
            server_address: args.server_address,
            server_port: args.server_port,
            log_level: args.log_level.into(),
            reward_address: args.reward_address,
            network_magic,
            secret: args.secret,
            node_socket_path: args.node_socket_path,
            node_bin_path: args.node_bin_path,
            mode: args.mode,
        })
    }

    fn get_network_magic(network: Network) -> Result<u64, AppError> {
        match network {
            Network::Mainnet => Ok(MAINNET_MAGIC),
            Network::Preprod => Ok(PREPROD_MAGIC),
            Network::Preview => Ok(PREVIEW_MAGIC),
            Network::Sanchonet => Ok(SANCHONET_MAGIC),
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
