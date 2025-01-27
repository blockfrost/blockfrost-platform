use crate::AppError;
use anyhow::{anyhow, Error, Result};
use clap::CommandFactory;
use clap::{arg, command, Parser, ValueEnum};
use inquire::validator::{ErrorMessage, Validation};
use inquire::{Confirm, Select, Text};
use pallas_network::miniprotocols::{MAINNET_MAGIC, PREPROD_MAGIC, PREVIEW_MAGIC};
use serde::{Deserialize, Serialize};
use std::fmt::{self, Formatter};
use std::fs;
use std::io::Write;
use std::path::PathBuf;
use tracing::Level;
use twelf::{config, Layer};

#[derive(Parser, Debug, Serialize)]
#[command(author, version, about, long_about = None)]
#[config]
pub struct Args {
    #[arg(long, default_value = "0.0.0.0")]
    server_address: String,

    #[arg(long, default_value = "3000")]
    server_port: u16,

    #[arg(long)]
    network: Option<Network>,

    #[arg(long, default_value = "info")]
    log_level: LogLevel,

    #[arg(long)]
    node_socket_path: Option<String>,

    #[arg(long, default_value = "compact")]
    mode: Mode,

    #[arg(long)]
    init: bool,

    /// Whether to run in solitary mode, without registering with the Icebreakers API
    #[arg(long)]
    solitary: bool,

    #[arg(long)]
    secret: Option<String>,

    #[arg(long)]
    reward_address: Option<String>,

    #[arg(long, default_value = "true", required = false)]
    metrics: bool,
}

fn get_config_path() -> PathBuf {
    dirs::config_dir()
        .expect("Could not determine config directory")
        .join("blockfrost")
        .join("config.toml")
}

impl Args {
    pub fn init() -> Result<Config, AppError> {
        let matches = Self::command().get_matches();
        let config_path = get_config_path();

        let mut config_layers = vec![
            Layer::Env(Some(String::from("BLOCKFROST_"))),
            Layer::Clap(matches),
        ];
        if config_path.exists() {
            config_layers.insert(0, Layer::Toml(config_path));
        }

        let arguments =
            Self::with_layers(&config_layers).map_err(|it| AppError::Server(it.to_string()))?;

        if arguments.init {
            Args::generate_config().map_err(|e| AppError::Server(e.to_string()))?;
        }

        Config::from_args(arguments)))
    }

    fn enum_prompt<T: std::fmt::Debug>(message: &str, enum_values: &[T]) -> Result<String> {
        Select::new(
            message,
            enum_values
                .iter()
                .map(|it| format!("{:?}", it))
                .collect::<Vec<_>>(),
        )
        .prompt()
        .map_err(|e| anyhow!(e))
    }

    fn to_file(&self, file_path: &PathBuf) -> Result<()> {
        let toml_string = toml::to_string(self).map_err(Error::new)?;
        let mut file = fs::File::create(file_path)?;
        file.write_all(toml_string.as_bytes())?;
        Ok(())
    }

    fn generate_config() -> Result<()> {
        let is_solitary = Confirm::new("Run in solitary mode?")
            .with_default(false)
            .with_help_message("Should be run without icebreakers API?")
            .prompt()?;

        let metrics = Confirm::new("Enable metrics?")
            .with_default(false)
            .with_help_message("Should metrics be enabled?")
            .prompt()?;

        let network = Args::enum_prompt(
            "Which network are you connecting to?",
            Network::value_variants(),
        )
        .and_then(|it| Network::from_str(it.as_str(), true).map_err(|e| anyhow!(e)))?;

        let mode = Args::enum_prompt("Mode?", Mode::value_variants())
            .and_then(|it| Mode::from_str(it.as_str(), true).map_err(|e| anyhow!(e)))?;

        let log_level =
            Args::enum_prompt("What should be the log level?", LogLevel::value_variants())
                .and_then(|it| LogLevel::from_str(it.as_str(), true).map_err(|e| anyhow!(e)))?;

        let server_address = Text::new("Enter the server IP address:")
            .with_default("0.0.0.0")
            .with_validator(|input: &str| {
                input
                    .parse::<std::net::IpAddr>()
                    .map(|_| Validation::Valid)
                    .or_else(|_| {
                        Ok(Validation::Invalid(ErrorMessage::Custom(
                            "Invalid IP address".into(),
                        )))
                    })
            })
            .prompt()?;

        let server_port = Text::new("Enter the port number:")
            .with_default("3000")
            .with_validator(|input: &str| match input.parse::<u16>() {
                Ok(port) if port >= 1 => Ok(Validation::Valid),
                _ => Ok(Validation::Invalid(ErrorMessage::Custom(
                    "Invalid port number. It must be between 1 and 65535".into(),
                ))),
            })
            .prompt()
            .map_err(|e| anyhow!(e))
            .and_then(|it| it.parse::<u16>().map_err(|e| anyhow!(e)))?;

        let node_socket_path = Text::new("Enter path to Cardano node socket:")
            .with_validator(|input: &str| {
                if input.is_empty() {
                    Ok(Validation::Invalid(ErrorMessage::Custom(
                        "Invalid path.".into(),
                    )))
                } else {
                    Ok(Validation::Valid)
                }
            })
            .prompt()?;

        let mut app_config = Args {
            init: false,
            solitary: is_solitary,
            network: Some(network),
            metrics,
            mode,
            log_level,
            server_address,
            server_port,
            node_socket_path: Some(node_socket_path),
            reward_address: None,
            secret: None,
        };

        if !is_solitary {
            let reward_address = Text::new("Enter the reward address:")
                .with_validator(|input: &str| {
                    if input.is_empty() {
                        Ok(Validation::Invalid(ErrorMessage::Custom(
                            "Invalid reward address.".into(),
                        )))
                    } else {
                        Ok(Validation::Valid)
                    }
                })
                .prompt()?;

            let secret = Text::new("Enter the icebreakers secret:")
                .with_validator(|input: &str| {
                    if input.is_empty() {
                        Ok(Validation::Invalid(ErrorMessage::Custom(
                            "Invalid reward address.".into(),
                        )))
                    } else {
                        Ok(Validation::Valid)
                    }
                })
                .prompt()?;
            app_config.reward_address = Some(reward_address);
            app_config.secret = Some(secret);
        }

        let config_path = get_config_path();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        app_config.to_file(&config_path)?;
        println!("Config has been written to {:?}", config_path);

        std::process::exit(0);
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
        let network = args
            .network
            .ok_or(AppError::Server("--network must be set".into()))?;
        let node_socket_path = args
            .node_socket_path
            .ok_or(AppError::Server("--node-socket-path must be set".into()))?;

        let network_magic = Self::get_network_magic(&network);

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
            None
        };

        Ok(Config {
            server_address: args.server_address,
            server_port: args.server_port,
            log_level: args.log_level.into(),
            network_magic,
            node_socket_path,
            mode: args.mode,
            icebreakers_config,
            max_pool_connections: 10,
            metrics: args.metrics,
            network,
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
