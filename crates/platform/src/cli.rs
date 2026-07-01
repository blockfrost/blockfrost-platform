use crate::config::{Config, Mode};
use anyhow::{Error, Result, anyhow};
use bf_common::{errors::AppError, types::LogLevel};
use clap::{CommandFactory, Parser, ValueEnum};
use inquire::validator::{ErrorMessage, Validation};
use inquire::{Confirm, Select, Text};
use serde::Serialize;
use std::fs;
use std::io::Write;
use std::net::IpAddr;
use std::path::PathBuf;
use twelf::{Layer, config};

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

    #[arg(long, default_value = "8192")]
    pub server_concurrency_limit: usize,

    #[arg(long, default_value_t = bf_common::DEFAULT_MAX_BODY_BYTES)]
    pub max_response_body_bytes: usize,

    #[arg(long, default_value = "info")]
    pub log_level: LogLevel,

    #[arg(long)]
    pub node_socket_path: Option<String>,

    #[arg(long, default_value = "compact")]
    pub mode: Mode,

    // `init` and `config` are runtime-only flags and are never written to the
    // generated config file (hence `skip_serializing`).
    #[arg(long, help = "Initialize a new configuration file")]
    #[serde(skip_serializing, default)]
    init: bool,

    #[arg(long, help = "Path to an existing configuration file")]
    #[serde(skip_serializing, default)]
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

    #[arg(long, help = "Path to a custom genesis configuration file")]
    pub custom_genesis_config: Option<PathBuf>,

    #[clap(long = "data-node")]
    pub data_node: Option<String>,

    #[clap(long = "data-node-timeout-sec", default_value = "30")]
    pub data_node_timeout: Option<u64>,

    /// Override the Gateway API URL (default: derived from network). Useful for
    /// self-hosted gateways or testing.
    #[arg(long)]
    pub gateway_url: Option<String>,

    /// A prefunded L1 key file for paying the Hydra transaction fees on L1, ~13 ADA per L2 cycle.
    #[arg(long)]
    pub hydra_cardano_signing_key: Option<PathBuf>,
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

        let enable_metrics = Confirm::new("Enable metrics?")
            .with_default(true)
            .with_help_message("Should metrics be enabled?")
            .prompt()?;

        let no_metrics = !enable_metrics;

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

        let (data_node, data_node_timeout) = {
            let data_node_url = Text::new("Data node URL (empty to skip):").prompt()?;

            if data_node_url.is_empty() {
                (None, None)
            } else {
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

                (Some(data_node_url), Some(data_node_timeout))
            }
        };

        let (reward_address, secret) = if is_solitary {
            (None, None)
        } else {
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

            (Some(reward_address), Some(secret))
        };

        let answers = ConfigAnswers {
            solitary: is_solitary,
            no_metrics,
            mode,
            log_level,
            server_address,
            server_port,
            node_socket_path,
            data_node,
            data_node_timeout,
            reward_address,
            secret,
        };

        let config_path = get_config_path();
        if let Some(parent) = config_path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        answers.into_args().to_file(&config_path)?;
        println!("\nConfig has been written to {config_path:?}");

        std::process::exit(0);
    }
}

/// The answers collected interactively by `--init`. Kept separate from the
/// prompting itself so that the (pure) mapping to [`Args`] can be unit-tested
/// without going through `inquire`.
#[derive(Debug, Clone)]
struct ConfigAnswers {
    solitary: bool,
    no_metrics: bool,
    mode: Mode,
    log_level: LogLevel,
    server_address: IpAddr,
    server_port: u16,
    node_socket_path: String,
    data_node: Option<String>,
    data_node_timeout: Option<u64>,
    reward_address: Option<String>,
    secret: Option<String>,
}

impl ConfigAnswers {
    /// Defaults for the fields `--init` doesn't prompt for.
    fn into_args(self) -> Args {
        Args {
            init: false,
            config: None,
            solitary: self.solitary,
            no_metrics: self.no_metrics,
            mode: self.mode,
            log_level: self.log_level,
            server_address: self.server_address,
            server_port: self.server_port,
            node_socket_path: Some(self.node_socket_path),
            reward_address: self.reward_address,
            secret: self.secret,
            custom_genesis_config: None,
            data_node: self.data_node,
            data_node_timeout: self.data_node_timeout,
            server_concurrency_limit: 8192,
            max_response_body_bytes: bf_common::DEFAULT_MAX_BODY_BYTES,
            gateway_url: None,
            hydra_cardano_signing_key: None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bf_common::types::Network;
    use futures::FutureExt;
    use futures::future::BoxFuture;
    use pretty_assertions::assert_eq;
    use tracing::Level; // for `.boxed()`

    fn mock_detector(_: &str) -> BoxFuture<'_, Result<Network, AppError>> {
        async { Ok(Network::Preview) }.boxed()
    }

    /// Builder for constructing cli arguments
    #[derive(Default)]
    struct TestArgsBuilder {
        node_socket_path: Option<String>,
        server_address: Option<String>,
        server_port: Option<u16>,
        server_concurrency_limit: Option<usize>,
        log_level: Option<String>,
        mode: Option<String>,
        solitary: bool,
        reward_address: Option<String>,
        secret: Option<String>,
        no_metrics: bool,
        data_node: Option<String>,
        data_node_timeout_sec: Option<String>,
        gateway_url: Option<String>,
        hydra_cardano_signing_key: Option<String>,
        max_response_body_bytes: Option<usize>,
    }

    impl TestArgsBuilder {
        fn new() -> Self {
            Self::default()
        }

        fn node_socket_path(mut self, path: &str) -> Self {
            self.node_socket_path = Some(path.to_string());
            self
        }

        fn server_address(mut self, addr: &str) -> Self {
            self.server_address = Some(addr.to_string());
            self
        }

        fn server_port(mut self, port: u16) -> Self {
            self.server_port = Some(port);
            self
        }

        fn server_concurrency_limit(mut self, limit: usize) -> Self {
            self.server_concurrency_limit = Some(limit);
            self
        }

        fn log_level(mut self, level: &str) -> Self {
            self.log_level = Some(level.to_string());
            self
        }

        fn mode(mut self, mode: &str) -> Self {
            self.mode = Some(mode.to_string());
            self
        }

        fn solitary(mut self) -> Self {
            self.solitary = true;
            self
        }

        fn reward_address(mut self, addr: &str) -> Self {
            self.reward_address = Some(addr.to_string());
            self
        }

        fn secret(mut self, secret: &str) -> Self {
            self.secret = Some(secret.to_string());
            self
        }

        fn no_metrics(mut self) -> Self {
            self.no_metrics = true;
            self
        }

        fn data_node(mut self, endpoint: &str) -> Self {
            self.data_node = Some(endpoint.to_string());
            self
        }

        fn data_node_timeout_sec(mut self, timeout: &str) -> Self {
            self.data_node_timeout_sec = Some(timeout.to_string());
            self
        }

        fn gateway_url(mut self, url: &str) -> Self {
            self.gateway_url = Some(url.to_string());
            self
        }

        fn hydra_cardano_signing_key(mut self, path: &str) -> Self {
            self.hydra_cardano_signing_key = Some(path.to_string());
            self
        }

        fn max_response_body_bytes(mut self, bytes: usize) -> Self {
            self.max_response_body_bytes = Some(bytes);
            self
        }

        fn build_args_vec(&self) -> Vec<String> {
            let mut args = vec!["testing".to_string()];

            let mut push_opt = |flag: &str, value: Option<String>| {
                if let Some(v) = value {
                    args.push(flag.to_string());
                    args.push(v);
                }
            };

            push_opt("--node-socket-path", self.node_socket_path.clone());
            push_opt("--server-address", self.server_address.clone());
            push_opt("--server-port", self.server_port.map(|p| p.to_string()));
            push_opt(
                "--server-concurrency-limit",
                self.server_concurrency_limit.map(|l| l.to_string()),
            );
            push_opt("--log-level", self.log_level.clone());
            push_opt("--mode", self.mode.clone());
            push_opt("--reward-address", self.reward_address.clone());
            push_opt("--secret", self.secret.clone());
            push_opt("--data-node", self.data_node.clone());
            push_opt(
                "--data-node-timeout-sec",
                self.data_node_timeout_sec.clone(),
            );
            push_opt("--gateway-url", self.gateway_url.clone());
            push_opt(
                "--hydra-cardano-signing-key",
                self.hydra_cardano_signing_key.clone(),
            );
            push_opt(
                "--max-response-body-bytes",
                self.max_response_body_bytes.map(|b| b.to_string()),
            );

            // optional parameters
            if self.solitary {
                args.push("--solitary".to_string());
            }

            if self.no_metrics {
                args.push("--no-metrics".to_string());
            }

            args
        }

        fn parse(self) -> Result<Args, clap::Error> {
            Args::try_parse_from(self.build_args_vec())
        }
    }

    #[tokio::test]
    async fn test_mandatory_ok() {
        let args = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .reward_address("test-reward-address")
            .secret("test-secret")
            .parse()
            .unwrap();

        let config = Config::from_args_with_detector(args, mock_detector)
            .await
            .expect("Config should be created successfully");

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
        let args = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .solitary()
            .parse()
            .unwrap();

        let config = Config::from_args_with_detector(args.clone(), mock_detector)
            .await
            .expect("Config should be created successfully");

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
        let args = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .reward_address("test-reward-address")
            .secret("test-secret")
            .no_metrics()
            .parse()
            .unwrap();

        let config = Config::from_args_with_detector(args, mock_detector)
            .await
            .expect("Config should be created successfully");

        assert!(config.no_metrics);
    }

    #[tokio::test]
    async fn test_non_defaults_ok() {
        let args = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .server_address("192.168.1.1")
            .server_port(5353)
            .log_level("debug")
            .mode("full")
            .no_metrics()
            .solitary()
            .parse()
            .unwrap();

        let config = Config::from_args_with_detector(args.clone(), mock_detector)
            .await
            .expect("Config should be created successfully");

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
        let args = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .reward_address("test-reward-address")
            .secret("test-secret")
            .solitary()
            .parse()
            .unwrap();

        let result = Config::from_args(args).await;

        assert!(result.is_err(), "Config creation should fail");
        assert_eq!(
            result.unwrap_err().to_string(),
            "Server startup error: Cannot set --reward-address or --secret in solitary mode (--solitary)"
        );
    }

    #[tokio::test]
    async fn test_data_node_cli_both_values() {
        let args = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .solitary()
            .data_node("http://localhost:3000")
            .data_node_timeout_sec("45")
            .parse()
            .unwrap();

        assert_eq!(args.data_node.as_deref(), Some("http://localhost:3000"));
        assert_eq!(args.data_node_timeout, Some(45));
    }

    #[tokio::test]
    async fn test_data_node_cli_only_endpoint_default_timeout() {
        let args = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .solitary()
            .data_node("http://localhost:8080")
            .parse()
            .unwrap();

        assert_eq!(args.data_node.as_deref(), Some("http://localhost:8080"));
        assert_eq!(args.data_node_timeout, Some(30));
    }

    #[tokio::test]
    async fn test_data_node_cli_absent_means_none_endpoint_and_default_timeout() {
        let args = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .solitary()
            .parse()
            .unwrap();

        assert!(args.data_node.is_none());
        assert_eq!(args.data_node_timeout, Some(30));
    }

    #[tokio::test]
    async fn test_data_node_cli_rejects_invalid_timeout() {
        let err = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .solitary()
            .data_node_timeout_sec("not-a-number")
            .parse()
            .unwrap_err();

        assert!(format!("{err}").contains("invalid value"));
    }

    #[tokio::test]
    async fn test_data_node_cli_all_params() {
        let args = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .solitary()
            .data_node("http://localhost:9000")
            .data_node_timeout_sec("60")
            .parse()
            .unwrap();

        assert_eq!(args.data_node.as_deref(), Some("http://localhost:9000"));
        assert_eq!(args.data_node_timeout, Some(60));
    }

    #[tokio::test]
    async fn test_data_node_config_created_correctly() {
        let args = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .solitary()
            .data_node("http://localhost:9000")
            .data_node_timeout_sec("60")
            .parse()
            .unwrap();

        let config = Config::from_args_with_detector(args, mock_detector)
            .await
            .unwrap();

        let data_node = config.data_node.expect("data_node should be Some");

        assert_eq!(data_node.endpoint, "http://localhost:9000");
        assert_eq!(
            data_node.request_timeout,
            std::time::Duration::from_secs(60)
        );
    }

    #[tokio::test]
    async fn test_data_node_config_none_when_not_provided() {
        let args = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .solitary()
            .parse()
            .unwrap();

        let config = Config::from_args_with_detector(args, mock_detector)
            .await
            .unwrap();

        assert!(config.data_node.is_none());
    }

    #[tokio::test]
    async fn test_server_concurrency_limit_default() {
        let args = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .solitary()
            .parse()
            .unwrap();

        let config = Config::from_args_with_detector(args, mock_detector)
            .await
            .unwrap();

        assert_eq!(config.server_concurrency_limit, 8192);
    }

    #[tokio::test]
    async fn test_server_concurrency_limit_custom() {
        let args = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .solitary()
            .server_concurrency_limit(512)
            .parse()
            .unwrap();

        let config = Config::from_args_with_detector(args, mock_detector)
            .await
            .unwrap();

        assert_eq!(config.server_concurrency_limit, 512);
    }

    #[tokio::test]
    async fn test_max_response_body_bytes_default() {
        let args = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .solitary()
            .parse()
            .unwrap();

        let config = Config::from_args_with_detector(args, mock_detector)
            .await
            .unwrap();

        assert_eq!(
            config.max_response_body_bytes,
            bf_common::DEFAULT_MAX_BODY_BYTES
        );
    }

    #[tokio::test]
    async fn test_max_response_body_bytes_custom() {
        let args = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .solitary()
            .max_response_body_bytes(1234)
            .parse()
            .unwrap();

        let config = Config::from_args_with_detector(args, mock_detector)
            .await
            .unwrap();

        assert_eq!(config.max_response_body_bytes, 1234);
    }

    #[tokio::test]
    async fn test_gateway_url_maps_into_icebreakers_config() {
        let args = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .reward_address("test-reward-address")
            .secret("test-secret")
            .gateway_url("https://gateway.example")
            .parse()
            .unwrap();

        let config = Config::from_args_with_detector(args, mock_detector)
            .await
            .unwrap();

        let icebreakers = config
            .icebreakers_config
            .expect("icebreakers_config should be Some in non-solitary mode");
        assert_eq!(
            icebreakers.gateway_url.as_deref(),
            Some("https://gateway.example")
        );
    }

    #[tokio::test]
    async fn test_gateway_url_absent_is_none() {
        let args = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .reward_address("test-reward-address")
            .secret("test-secret")
            .parse()
            .unwrap();

        let config = Config::from_args_with_detector(args, mock_detector)
            .await
            .unwrap();

        let icebreakers = config.icebreakers_config.expect("should be Some");
        assert!(icebreakers.gateway_url.is_none());
    }

    #[tokio::test]
    async fn test_hydra_signing_key_maps_to_hydra_config() {
        let args = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .solitary()
            .hydra_cardano_signing_key("/path/to/key.skey")
            .parse()
            .unwrap();

        let config = Config::from_args_with_detector(args, mock_detector)
            .await
            .unwrap();

        let hydra = config.hydra.expect("hydra config should be Some");
        assert_eq!(
            hydra.cardano_signing_key,
            std::path::PathBuf::from("/path/to/key.skey")
        );
    }

    #[tokio::test]
    async fn test_hydra_absent_is_none() {
        let args = TestArgsBuilder::new()
            .node_socket_path("/path/to/socket")
            .solitary()
            .parse()
            .unwrap();

        let config = Config::from_args_with_detector(args, mock_detector)
            .await
            .unwrap();

        assert!(config.hydra.is_none());
    }

    #[test]
    fn test_config_answers_into_args_solitary() {
        let answers = ConfigAnswers {
            solitary: true,
            no_metrics: true,
            mode: Mode::Full,
            log_level: LogLevel::Debug,
            server_address: "127.0.0.1".parse().unwrap(),
            server_port: 4321,
            node_socket_path: "/sock".to_string(),
            data_node: Some("http://localhost:3010".to_string()),
            data_node_timeout: Some(45),
            reward_address: None,
            secret: None,
        };

        let args = answers.into_args();

        // prompted values are carried over
        assert!(args.solitary);
        assert!(args.no_metrics);
        assert_eq!(args.mode, Mode::Full);
        assert_eq!(args.server_address.to_string(), "127.0.0.1");
        assert_eq!(args.server_port, 4321);
        assert_eq!(args.node_socket_path.as_deref(), Some("/sock"));
        assert_eq!(args.data_node.as_deref(), Some("http://localhost:3010"));
        assert_eq!(args.data_node_timeout, Some(45));
        assert!(args.reward_address.is_none());
        assert!(args.secret.is_none());

        // non-interactive defaults are applied
        assert_eq!(args.server_concurrency_limit, 8192);
        assert_eq!(
            args.max_response_body_bytes,
            bf_common::DEFAULT_MAX_BODY_BYTES
        );
        assert!(args.gateway_url.is_none());
        assert!(args.hydra_cardano_signing_key.is_none());
        assert!(args.custom_genesis_config.is_none());
    }

    #[test]
    fn test_config_answers_into_args_no_data_node() {
        let answers = ConfigAnswers {
            solitary: false,
            no_metrics: false,
            mode: Mode::Compact,
            log_level: LogLevel::Info,
            server_address: "0.0.0.0".parse().unwrap(),
            server_port: 3000,
            node_socket_path: "/sock".to_string(),
            data_node: None,
            data_node_timeout: None,
            reward_address: Some("addr".to_string()),
            secret: Some("sec".to_string()),
        };

        let args = answers.into_args();

        assert!(args.data_node.is_none());
        // no sentinel timeout is emitted when there is no data node
        assert!(args.data_node_timeout.is_none());
        assert_eq!(args.reward_address.as_deref(), Some("addr"));
        assert_eq!(args.secret.as_deref(), Some("sec"));
    }
}
