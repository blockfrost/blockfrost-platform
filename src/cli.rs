use anyhow::{anyhow, Result};
use clap::{arg, command, Parser, Subcommand, ValueEnum};
use inquire::{
    validator::{ErrorMessage, Validation},
    Confirm, Select, Text,
};
use pallas_network::miniprotocols::{MAINNET_MAGIC, PREPROD_MAGIC, PREVIEW_MAGIC};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::{self, Write};
use std::{
    fmt::{self, Formatter},
    path::PathBuf,
};
use toml;
use tracing::Level;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    Run(RunArgs),
    Init,
}

fn default_ip() -> String {
    "0.0.0.0".to_string()
}
fn default_port() -> u16 {
    3000
}

#[derive(Parser, Debug, Serialize, Deserialize)]
#[command(author, version, about, long_about = None)]
pub struct RunArgs {
    #[serde(default = "default_ip")]
    #[arg(long, default_value = "0.0.0.0")]
    server_address: String,

    #[serde(default = "default_port")]
    #[arg(long, default_value = "3000")]
    server_port: u16,

    #[arg(long, required_unless_present = "config")]
    network: Option<Network>,

    #[serde(default = "LogLevel::default_log_level")]
    #[arg(long, default_value = "info")]
    log_level: LogLevel,

    #[arg(long, required_unless_present = "config")]
    node_socket_path: Option<String>,

    #[serde(default = "Mode::default_mode")]
    #[arg(long, default_value = "compact")]
    mode: Mode,

    /// Whether to run in solitary mode, without registering with the Icebreakers API
    #[serde(default)]
    #[arg(long)]
    solitary: bool,

    #[serde(skip)]
    #[arg(long)]
    config: Option<PathBuf>,

    #[arg(
        long,
        required_unless_present_any(&["solitary", "config"]),
        conflicts_with("solitary"),
        requires("reward_address")
    )]
    secret: Option<String>,

    #[arg(
        long,
        required_unless_present_any(&["solitary", "config"]),
        conflicts_with("solitary"),
        requires("secret")
    )]
    reward_address: Option<String>,
}

impl RunArgs {
    pub fn from_file(&self) -> Result<Self> {
        let contents = fs::read_to_string(self.config.clone().unwrap())?;
        let from_file: RunArgs = toml::from_str(&contents)?;
        Ok(merge_struct::merge(&from_file, self)?)
    }

    pub fn to_file(&self, file_path: &PathBuf) -> Result<()> {
        let toml_string =
            toml::to_string(self).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        let mut file = fs::File::create(file_path)?;
        file.write_all(toml_string.as_bytes())?;
        Ok(())
    }

    fn get_network_magic(network: &Network) -> u64 {
        match network {
            Network::Mainnet => MAINNET_MAGIC,
            Network::Preprod => PREPROD_MAGIC,
            Network::Preview => PREVIEW_MAGIC,
        }
    }

    pub fn to_config(&self) -> Config {
        let network = self.network.clone();
        let network_magic = Self::get_network_magic(&network.clone().unwrap());

        let icebreakers_config = match (&self.solitary, &self.reward_address, &self.secret) {
            (true, Some(reward_address), Some(secret)) => Some(IcebreakersConfig {
                reward_address: reward_address.clone(),
                secret: secret.clone(),
            }),
            _ => None,
        };

        Config {
            server_address: self.server_address.clone(),
            server_port: self.server_port,
            log_level: self.log_level.clone().into(),
            network_magic,
            node_socket_path: self.node_socket_path.clone().unwrap(),
            mode: self.mode.clone(),
            icebreakers_config,
            max_pool_connections: 10,
            network: network.unwrap(),
        }
    }
}

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize)]
pub enum Mode {
    Compact,
    Light,
    Full,
}

impl Mode {
    pub fn default_mode() -> Mode {
        Mode::Compact
    }
}

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize)]
pub enum Network {
    Mainnet,
    Preprod,
    Preview,
}

#[derive(Debug, Clone, ValueEnum, Serialize, Deserialize)]
pub enum LogLevel {
    Debug,
    Info,
    Warn,
    Error,
    Trace,
}

impl LogLevel {
    pub fn default_log_level() -> LogLevel {
        LogLevel::Info
    }
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
}

pub struct IcebreakersConfig {
    pub reward_address: String,
    pub secret: String,
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

impl Config {
    pub fn init(cli: Cli) -> Result<Self> {
        match cli.command {
            Commands::Init => {
                Self::generate_config()?;
                panic!("Expected `Commands::Run`, but got `Commands::Init`.");
            }
            Commands::Run(args) => {
                let config_path = if let Some(ref path) = args.config {
                    if !path.exists() {
                        println!("Config file does not exist");
                        std::process::exit(1);
                    } else {
                        args.from_file()?
                    }
                } else {
                    args
                };
                Ok(config_path.to_config())
            }
        }
    }

    fn generate_config() -> Result<()> {
        let is_solitary = Confirm::new("Run in solitary mode?")
            .with_default(false)
            .with_help_message("Should be run without icebreakers API?")
            .prompt()?;

        let network = enum_prompt(
            "Which network are you connecting to?",
            Network::value_variants(),
        )
        .and_then(|it| Network::from_str(it.as_str(), true).map_err(|e| anyhow!(e)))?;

        let mode = enum_prompt("Mode?", Mode::value_variants())
            .and_then(|it| Mode::from_str(it.as_str(), true).map_err(|e| anyhow!(e)))?;

        let log_level = enum_prompt("What should be the log level?", LogLevel::value_variants())
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

        let mut app_config = RunArgs {
            config: None,
            solitary: is_solitary,
            network: Some(network),
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

fn get_config_path() -> PathBuf {
    dirs::config_dir()
        .expect("Could not determine config directory")
        .join("blockfrost")
        .join("config.toml")
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
