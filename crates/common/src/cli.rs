use crate::{
    config::{Config, Mode},
    errors::AppError,
    types::LogLevel,
};
use anyhow::{Error, Result, anyhow};
use clap::{CommandFactory, Parser, ValueEnum};
use inquire::validator::{ErrorMessage, Validation};
use inquire::{Confirm, Select, Text};
use serde::{Deserialize, Serialize};
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

#[derive(Debug, Clone, Copy, ValueEnum, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "lowercase")]
pub enum DataNodeType {
    #[default]
    Dolos,
}

impl std::fmt::Display for DataNodeType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DataNodeType::Dolos => write!(f, "dolos"),
        }
    }
}

#[derive(Parser, Debug, Clone, Serialize, Deserialize, Default)]
pub struct DataNodeArgs {
    pub node_type: DataNodeType,
    pub endpoint: Option<String>,
    pub request_timeout: u64,
}

#[derive(Parser, Debug, Serialize, Clone)]
#[command(author,
          name = "blockfrost-platform", // otherwise it’s `common`
          bin_name = "blockfrost-platform",
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

    #[arg(long, help = "Path to an configuration file")]
    pub custom_genesis_config: Option<PathBuf>,

    #[clap(long = "data-node")]
    pub data_node: Option<String>,

    #[clap(long = "data-node-type", default_value = "dolos")]
    pub data_node_type: DataNodeType,

    #[clap(long = "data-node-timeout-sec", default_value = "30")]
    pub data_node_timeout: Option<u64>,
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

    pub async fn init() -> Result<Config, AppError> {
        let initial_args = Args::parse();
        let config_path = initial_args.config.unwrap_or(get_config_path());

        let arguments = Args::parse_args(config_path)?;

        SHOULD_SKIP_SERIALIZNG_FIELDS.store(true, Ordering::SeqCst);

        if arguments.init {
            Args::generate_config().map_err(|e| AppError::Server(e.to_string()))?;
        }

        match arguments.config {
            Some(path) => {
                let parsed_args = Args::parse_args(path)?;

                Config::from_args(parsed_args).await
            },
            None => Config::from_args(arguments).await,
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
                .map(|it| format!("{it:?}"))
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

        let no_metrics = Confirm::new("Enable metrics?")
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

        let data_node = {
            let data_node_url = Text::new("Data node URL (empty to skip):").prompt()?;

            if data_node_url.is_empty() {
                DataNodeArgs {
                    node_type: DataNodeType::default(),
                    endpoint: None,
                    request_timeout: 0,
                }
            } else {
                let data_node_type =
                    Args::enum_prompt("Data node type?", DataNodeType::value_variants(), 0)
                        .and_then(|it| {
                            DataNodeType::from_str(it.as_str(), true).map_err(|e| anyhow!(e))
                        })?;

                let data_node_timeout = Text::new("Data node timeout (s):")
                    .with_default("30")
                    .with_validator(|i: &str| match i.parse::<u64>() {
                        Ok(t) if t > 0 => Ok(Validation::Valid),
                        _ => Ok(Validation::Invalid(ErrorMessage::Custom(
                            "Must be > 0".into(),
                        ))),
                    })
                    .prompt()?
                    .parse()?;

                DataNodeArgs {
                    node_type: data_node_type,
                    endpoint: Some(data_node_url),
                    request_timeout: data_node_timeout,
                }
            }
        };

        let mut app_config = Args {
            init: false,
            config: None,
            solitary: is_solitary,
            no_metrics,
            mode,
            log_level,
            server_address,
            server_port,
            node_socket_path: Some(node_socket_path),
            reward_address: None,
            secret: None,
            custom_genesis_config: None,
            data_node: data_node.endpoint,
            data_node_type: data_node.node_type,
            data_node_timeout: Some(data_node.request_timeout),
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
                            "Invalid secret.".into(),
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
        println!("\nConfig has been written to {config_path:?}");

        std::process::exit(0);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Network;
    use futures::FutureExt;
    use futures::future::BoxFuture;
    use pretty_assertions::assert_eq;
    use tracing::Level; // for `.boxed()`

    fn mock_detector(_: &str) -> BoxFuture<'_, Result<Network, AppError>> {
        async { Ok(Network::Preview) }.boxed()
    }

    #[tokio::test]
    async fn test_mandatory_ok() {
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
        let maybe_config = Config::from_args_with_detector(args, mock_detector).await;

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
        assert_eq!(config.no_metrics, false);
        assert!(config.icebreakers_config.is_some());

        let icebreaker_config = config.icebreakers_config.unwrap();
        assert_eq!(icebreaker_config.reward_address, "test-reward-address");
        assert_eq!(icebreaker_config.secret, "test-secret");
    }

    #[tokio::test]
    async fn test_mandatory_solitary_ok() {
        let inputs = vec![
            "testing",
            "--node-socket-path",
            "/path/to/socket",
            "--solitary",
        ];

        let args = Args::try_parse_from(inputs).unwrap();
        let maybe_config = Config::from_args_with_detector(args.clone(), mock_detector).await;

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
        assert_eq!(config.no_metrics, false);
        assert!(config.icebreakers_config.is_none());
        assert!(args.solitary);
    }

    #[tokio::test]
    async fn test_no_metrics_ok() {
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

        let maybe_config = Config::from_args_with_detector(args.clone(), mock_detector).await;

        assert!(
            maybe_config.is_ok(),
            "Config should be created successfully"
        );

        assert!(maybe_config.unwrap().no_metrics);
    }

    #[tokio::test]
    async fn test_non_defaults_ok() {
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

        let maybe_config = Config::from_args_with_detector(args.clone(), mock_detector).await;

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

    #[tokio::test]
    async fn test_solitary_conflict_fail() {
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

        let maybe_config = Config::from_args(args.clone()).await;

        assert!(
            maybe_config.is_err(),
            "Config should be created successfully"
        );

        assert_eq!(maybe_config.unwrap_err().to_string(), "Server startup error: Cannot set --reward-address or --secret in solitary mode (--solitary)".to_string());
    }

    #[tokio::test]
    async fn test_data_node_cli_both_values() {
        let inputs = vec![
            "testing",
            "--node-socket-path",
            "/path/to/socket",
            "--solitary",
            "--data-node",
            "http://localhost:3000",
            "--data-node-timeout-sec",
            "45",
        ];

        let args = Args::try_parse_from(inputs).unwrap();
        assert_eq!(args.data_node.as_deref(), Some("http://localhost:3000"));
        assert_eq!(args.data_node_timeout, Some(45));
    }

    #[tokio::test]
    async fn test_data_node_cli_only_endpoint_default_timeout() {
        let inputs = vec![
            "testing",
            "--node-socket-path",
            "/path/to/socket",
            "--solitary",
            "--data-node",
            "http://localhost:8080",
        ];

        let args = Args::try_parse_from(inputs).unwrap();
        assert_eq!(args.data_node.as_deref(), Some("http://localhost:8080"));
        assert_eq!(args.data_node_timeout, Some(30));
    }

    #[tokio::test]
    async fn test_data_node_cli_absent_means_none_endpoint_and_default_timeout() {
        let inputs = vec![
            "testing",
            "--node-socket-path",
            "/path/to/socket",
            "--solitary",
        ];

        let args = Args::try_parse_from(inputs).unwrap();
        assert!(args.data_node.is_none());
        assert_eq!(args.data_node_timeout, Some(30));
    }

    #[tokio::test]
    async fn test_data_node_cli_rejects_invalid_timeout() {
        let inputs = vec![
            "testing",
            "--node-socket-path",
            "/path/to/socket",
            "--solitary",
            "--data-node-timeout-sec",
            "not-a-number",
        ];

        let err = Args::try_parse_from(inputs).unwrap_err();
        assert!(format!("{err}").contains("invalid value"));
    }

    #[tokio::test]
    async fn test_data_node_cli_all_params() {
        let inputs = vec![
            "testing",
            "--node-socket-path",
            "/path/to/socket",
            "--solitary",
            "--data-node",
            "http://localhost:9000",
            "--data-node-type",
            "dolos",
            "--data-node-timeout-sec",
            "60",
        ];

        let args = Args::try_parse_from(inputs).unwrap();

        assert_eq!(args.data_node.as_deref(), Some("http://localhost:9000"));
        assert_eq!(args.data_node_type, DataNodeType::Dolos);
        assert_eq!(args.data_node_timeout, Some(60));
    }

    #[tokio::test]
    async fn test_data_node_config_created_correctly() {
        let inputs = vec![
            "testing",
            "--node-socket-path",
            "/path/to/socket",
            "--solitary",
            "--data-node",
            "http://localhost:9000",
            "--data-node-type",
            "dolos",
            "--data-node-timeout-sec",
            "60",
        ];

        let args = Args::try_parse_from(inputs).unwrap();
        let config = Config::from_args_with_detector(args, mock_detector).await.unwrap();

        let data_node = config.data_node.expect("data_node should be Some");

        assert_eq!(data_node.endpoint, "http://localhost:9000");
        assert_eq!(data_node.node_type, DataNodeType::Dolos);
        assert_eq!(data_node.request_timeout, std::time::Duration::from_secs(60));
    }

    #[tokio::test]
    async fn test_data_node_config_none_when_not_provided() {
        let inputs = vec![
            "testing",
            "--node-socket-path",
            "/path/to/socket",
            "--solitary",
        ];

        let args = Args::try_parse_from(inputs).unwrap();
        let config = Config::from_args_with_detector(args, mock_detector).await.unwrap();

        assert!(config.data_node.is_none());
    }
}
