use crate::AppError;
use crate::config::{Config, LogLevel, Mode};
use anyhow::{Error, Result, anyhow};
use clap::{CommandFactory, ValueEnum};
use clap::{Parser, arg, command};
use inquire::validator::{ErrorMessage, Validation};
use inquire::{Confirm, Select, Text};
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::net::IpAddr;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use twelf::{Layer, config};

static SHOULD_SKIP_SERIALIZNG_FIELDS: AtomicBool = AtomicBool::new(false);

fn should_skip_serializng_fields<T>(_: &T) -> bool {
    SHOULD_SKIP_SERIALIZNG_FIELDS.load(Ordering::SeqCst)
}

#[derive(Parser, Debug, Serialize, Clone)]
#[command(author,
          version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_REVISION"), ")"),
          about,
          long_about = None)]
#[config]
pub struct Args {
    #[arg(long, default_value = "0.0.0.0")]
    pub server_address: IpAddr,

    #[arg(long, default_value = "3000")]
    pub server_port: u16,

    #[arg(long, default_value = "info")]
    pub log_level: LogLevel,

    #[arg(long)]
    pub node_socket_path: Option<String>,

    #[arg(long, default_value = "compact")]
    pub mode: Mode,

    #[arg(long, help = "Initialize a new configuration file")]
    #[serde(skip_serializing_if = "should_skip_serializng_fields")]
    #[serde(default)]
    init: bool,

    #[arg(long, help = "Path to an existing configuration file")]
    #[serde(skip_serializing_if = "should_skip_serializng_fields")]
    config: Option<PathBuf>,

    /// Whether to run in solitary mode, without registering with the Icebreakers API
    #[arg(long)]
    pub solitary: bool,

    #[arg(long)]
    pub secret: Option<String>,

    #[arg(long)]
    pub reward_address: Option<String>,

    #[arg(long)]
    pub no_metrics: bool,
}

fn get_config_path() -> PathBuf {
    dirs::config_dir()
        .expect("Could not determine config directory")
        .join("blockfrost-platform")
        .join("config.toml")
}

impl Args {
    fn parse_args(config_path: PathBuf) -> Result<Args, AppError> {
        const ENV_PREFIX: &str = "BLOCKFROST_";

        let no_config_file = !config_path.exists();
        let no_env_vars = std::env::vars().all(|(key, _val)| !key.starts_with(ENV_PREFIX));
        let empty_argv = std::env::args().len() == 1;
        if no_config_file && no_env_vars && empty_argv {
            Self::command().print_help().unwrap();
            std::process::exit(1);
        }
        let matches = Self::command().get_matches();

        let mut config_layers = vec![
            Layer::Env(Some(String::from(ENV_PREFIX))),
            Layer::Clap(matches),
        ];
        if config_path.exists() {
            config_layers.insert(0, Layer::Toml(config_path.clone()));
        }

        Self::with_layers(&config_layers).map_err(|e| match e {
            twelf::Error::Toml(_) => AppError::Server(format!(
                "Failed to parse config file '{}'",
                config_path.to_string_lossy()
            )),
            _ => AppError::Server(e.to_string()),
        })
    }

    pub fn init() -> Result<Config, AppError> {
        let initial_args = Args::parse();
        let config_path = initial_args.config.unwrap_or(get_config_path());

        let arguments = Args::parse_args(config_path)?;

        SHOULD_SKIP_SERIALIZNG_FIELDS.store(true, Ordering::SeqCst);

        if arguments.init {
            Args::generate_config().map_err(|e| AppError::Server(e.to_string()))?;
        }

        match arguments.config {
            Some(path) => Config::from_args(Args::parse_args(path)?),
            None => Config::from_args(arguments),
        }
    }

    fn enum_prompt<T: std::fmt::Debug>(
        message: &str,
        enum_values: &[T],
        starting_cursor: usize,
    ) -> Result<String> {
        Select::new(
            message,
            enum_values
                .iter()
                .map(|it| format!("{:?}", it))
                .collect::<Vec<_>>(),
        )
        .with_starting_cursor(starting_cursor)
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
            .with_default(true)
            .with_help_message("Should metrics be enabled?")
            .prompt()?;

        let mode = Args::enum_prompt("Mode?", Mode::value_variants(), 0)
            .and_then(|it| Mode::from_str(it.as_str(), true).map_err(|e| anyhow!(e)))?;

        let log_level = Args::enum_prompt(
            "What should be the log level?",
            LogLevel::value_variants(),
            1,
        )
        .and_then(|it| LogLevel::from_str(it.as_str(), true).map_err(|e| anyhow!(e)))?;

        // TODO: Maybe use [`inquire::CustomType`]?
        let server_address: IpAddr = Text::new("Enter the server IP address:")
            .with_default("0.0.0.0")
            .with_validator(|input: &str| {
                input
                    .parse::<IpAddr>()
                    .map(|_| Validation::Valid)
                    .or_else(|_| {
                        Ok(Validation::Invalid(ErrorMessage::Custom(
                            "Invalid IP address".into(),
                        )))
                    })
            })
            .prompt()?
            .parse()?;

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
            config: None,
            solitary: is_solitary,
            no_metrics: !metrics,
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
        println!("\nConfig has been written to {:?}", config_path);

        std::process::exit(0);
    }
}

#[cfg(test)]
mod tests {
    use tracing::Level;

    use crate::config::Network;

    use super::*;

    fn mock_detector(_: &str) -> Result<Network, AppError> {
        Ok(Network::Preview)
    }

    #[test]
    fn test_mandatory_ok() {
        let inputs = vec![
            "testing",
            "--node-socket-path",
            "/path/to/socket",
            "--reward-address",
            "test-reward-address",
            "--secret",
            "test-secret",
        ];

        let args = Args::try_parse_from(inputs).unwrap();

        let maybe_config = Config::from_args_with_detector(args, mock_detector);

        assert!(
            maybe_config.is_ok(),
            "Config should be created successfully"
        );

        let config = maybe_config.unwrap();

        // Test mandatory values are properly set with minimal configuration
        assert_eq!(config.node_socket_path, "/path/to/socket");
        assert_eq!(config.max_pool_connections, 10);
        assert_eq!(config.server_address.to_string(), "0.0.0.0");
        assert_eq!(config.server_port, 3000);
        assert_eq!(config.log_level, Level::INFO);
        assert_eq!(config.mode, Mode::Compact);
        assert!(!config.no_metrics);
        assert!(config.icebreakers_config.is_some());

        let icebreaker_config = config.icebreakers_config.unwrap();
        assert_eq!(icebreaker_config.reward_address, "test-reward-address");
        assert_eq!(icebreaker_config.secret, "test-secret");
    }

    #[test]
    fn test_mandatory_solitary_ok() {
        let inputs = vec![
            "testing",
            "--node-socket-path",
            "/path/to/socket",
            "--solitary",
        ];

        let args = Args::try_parse_from(inputs).unwrap();

        let maybe_config = Config::from_args_with_detector(args.clone(), mock_detector);

        assert!(
            maybe_config.is_ok(),
            "Config should be created successfully"
        );

        let config = maybe_config.unwrap();

        // Test mandatory values are properly set with minimal configuration
        assert_eq!(config.node_socket_path, "/path/to/socket");
        assert_eq!(config.max_pool_connections, 10);
        assert_eq!(config.server_address.to_string(), "0.0.0.0");
        assert_eq!(config.server_port, 3000);
        assert_eq!(config.log_level, Level::INFO);
        assert_eq!(config.mode, Mode::Compact);
        assert!(!config.no_metrics);
        assert!(config.icebreakers_config.is_none());
        assert!(args.solitary);
    }

    #[test]
    fn test_no_metrics_ok() {
        let inputs = vec![
            "testing",
            "--node-socket-path",
            "/path/to/socket",
            "--reward-address",
            "test-reward-address",
            "--secret",
            "test-secret",
            "--no-metrics",
        ];

        let args = Args::try_parse_from(inputs).unwrap();

        let maybe_config = Config::from_args_with_detector(args.clone(), mock_detector);

        assert!(
            maybe_config.is_ok(),
            "Config should be created successfully"
        );

        assert!(maybe_config.unwrap().no_metrics);
    }

    #[test]
    fn test_non_defaults_ok() {
        let inputs = vec![
            "testing",
            "--node-socket-path",
            "/path/to/socket",
            "--server-address",
            "192.168.1.1",
            "--server-port",
            "5353",
            "--log-level",
            "debug",
            "--mode",
            "full",
            "--no-metrics",
            "--solitary",
        ];

        let args = Args::try_parse_from(inputs).unwrap();

        let maybe_config = Config::from_args_with_detector(args.clone(), mock_detector);

        assert!(
            maybe_config.is_ok(),
            "Config should be created successfully"
        );

        let config = maybe_config.unwrap();

        // Test mandatory values are properly set with minimal configuration
        assert_eq!(config.node_socket_path, "/path/to/socket");
        assert_eq!(config.max_pool_connections, 10);
        assert_eq!(config.server_address.to_string(), "192.168.1.1");
        assert_eq!(config.server_port, 5353);
        assert_eq!(config.log_level, Level::DEBUG);
        assert_eq!(config.mode, Mode::Full);
        assert!(config.no_metrics);
        assert!(config.icebreakers_config.is_none());
        assert!(args.solitary);
    }

    #[test]
    fn test_solitary_conflict_fail() {
        let inputs = vec![
            "testing",
            "--node-socket-path",
            "/path/to/socket",
            "--reward-address",
            "test-reward-address",
            "--secret",
            "test-secret",
            "--solitary",
        ];

        let args = Args::try_parse_from(inputs).unwrap();

        let maybe_config = Config::from_args(args.clone());

        assert!(
            maybe_config.is_err(),
            "Config should be created successfully"
        );

        assert_eq!(maybe_config.unwrap_err().to_string(), "Server startup error: Cannot set --reward-address or --secret in solitary mode (--solitary)".to_string());
    }
}
