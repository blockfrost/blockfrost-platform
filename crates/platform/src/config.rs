use crate::cli::Args;
use crate::genesis::{GenesisRegistry, genesis};
use bf_api_provider::types::GenesisResponse;
use bf_common::errors::AppError;
use bf_common::types::Network;
use clap::ValueEnum;
use futures::FutureExt; // for `.boxed()`
use futures::future::BoxFuture;
use pallas_network::facades::NodeClient;
use serde::{Deserialize, Serialize};
use std::fmt::{self, Formatter};
use std::fs;
use std::path::PathBuf;
use std::time::Duration;
use tracing::Level;

#[derive(Clone, Debug)]
pub struct Config {
    pub server_address: std::net::IpAddr,
    pub server_port: u16,
    pub server_concurrency_limit: usize,
    pub max_response_body_bytes: usize,
    pub log_level: Level,
    pub node_socket_path: String,
    pub mode: Mode,
    pub icebreakers_config: Option<IcebreakersConfig>,
    pub max_pool_connections: usize,
    pub no_metrics: bool,
    pub network: Network,
    pub custom_genesis_config: Option<PathBuf>,
    pub data_node: Option<DataNodeConfig>,
    pub hydra: Option<HydraConfig>,
}

#[derive(Clone, Deserialize, Debug)]
pub struct DataNodeConfig {
    pub endpoint: String,
    pub request_timeout: Duration,
}

#[derive(Clone, Debug)]
pub struct IcebreakersConfig {
    pub reward_address: String,
    pub secret: String,
    pub gateway_url: Option<String>,
}

#[derive(Clone, Debug)]
pub struct HydraConfig {
    pub cardano_signing_key: PathBuf,
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

impl Config {
    pub async fn from_args_with_detector(
        args: Args,
        detector: impl for<'a> Fn(&'a str) -> BoxFuture<'a, Result<Network, AppError>>,
    ) -> Result<Self, AppError> {
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
                gateway_url: args.gateway_url.clone(),
            })
        } else {
            if args.reward_address.is_some() || args.secret.is_some() {
                return Err(AppError::Server(
                    "Cannot set --reward-address or --secret in solitary mode (--solitary)".into(),
                ));
            }
            None
        };

        let network = detector(&node_socket_path).await?;

        let data_node = args.data_node.map(|endpoint| {
            let timeout = Duration::from_secs(args.data_node_timeout.unwrap_or(30));

            DataNodeConfig {
                endpoint,
                request_timeout: timeout,
            }
        });

        let hydra = args
            .hydra_cardano_signing_key
            .map(|cardano_signing_key| HydraConfig {
                cardano_signing_key,
            });

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
            custom_genesis_config: args.custom_genesis_config,
            data_node,
            hydra,
            server_concurrency_limit: args.server_concurrency_limit,
            max_response_body_bytes: args.max_response_body_bytes,
        })
    }

    pub async fn from_args(args: Args) -> Result<Self, AppError> {
        Self::from_args_with_detector(args, |s| detect_network(s).boxed()).await
    }

    /// Build the full genesis registry, overriding or prepending
    /// a user-supplied file if `custom_genesis_config` is Some.
    pub fn with_custom_genesis(&self) -> Result<Vec<(Network, GenesisResponse)>, AppError> {
        let mut registry = genesis();

        // if user pointed us at a file, load & insert it
        if let Some(path) = &self.custom_genesis_config {
            let data = fs::read_to_string(path).map_err(|e| {
                AppError::Server(format!(
                    "Failed to read custom genesis file {}: {}",
                    path.display(),
                    e
                ))
            })?;

            // try JSON and TOML
            let custom: GenesisResponse = serde_json::from_str(&data)
                .or_else(|_| toml::from_str(&data))
                .map_err(|e| {
                    AppError::Server(format!(
                        "Failed to parse custom genesis file {}: {}",
                        path.display(),
                        e
                    ))
                })?;

            // prepend or replace the entry for custom network
            registry.add(Network::Custom, custom);
        }

        Ok(registry)
    }
}

async fn detect_network(socket_path: &str) -> Result<Network, AppError> {
    let all_magics = genesis().all_magics();

    for magic in all_magics {
        let ok = match NodeClient::connect(&socket_path, magic).await {
            Ok(conn) => {
                conn.abort().await;
                true
            },
            Err(_) => false,
        };

        if ok {
            return Ok(genesis().network_by_magic(magic).clone());
        }
    }

    Err(AppError::Server(format!(
        "Could not detect network from '{socket_path}' is the node running?"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    const CUSTOM_MAGIC: u64 = 999_999;

    /// A minimal but valid genesis payload with a distinctive network magic.
    fn custom_genesis_json() -> String {
        format!(
            r#"{{
                "active_slots_coefficient": 0.05,
                "update_quorum": 5,
                "max_lovelace_supply": "45000000000000000",
                "network_magic": {CUSTOM_MAGIC},
                "epoch_length": 432000,
                "system_start": 1506203091,
                "slots_per_kes_period": 129600,
                "slot_length": 1,
                "max_kes_evolutions": 62,
                "security_param": 2160
            }}"#
        )
    }

    fn write_temp(name: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(name);
        let mut file = fs::File::create(&path).expect("create temp file");
        file.write_all(contents.as_bytes())
            .expect("write temp file");
        path
    }

    fn base_config(custom_genesis_config: Option<PathBuf>) -> Config {
        Config {
            server_address: "0.0.0.0".parse().unwrap(),
            server_port: 3000,
            server_concurrency_limit: 8192,
            max_response_body_bytes: bf_common::DEFAULT_MAX_BODY_BYTES,
            log_level: Level::INFO,
            node_socket_path: "/tmp/socket".to_string(),
            mode: Mode::Compact,
            icebreakers_config: None,
            max_pool_connections: 10,
            no_metrics: false,
            network: Network::Preview,
            custom_genesis_config,
            data_node: None,
            hydra: None,
        }
    }

    #[test]
    fn with_custom_genesis_none_returns_defaults() {
        let config = base_config(None);
        let registry = config.with_custom_genesis().unwrap();

        assert_eq!(registry.len(), genesis().len());
        assert!(!registry.iter().any(|(n, _)| *n == Network::Custom));
    }

    #[test]
    fn with_custom_genesis_json_is_added() {
        let path = write_temp("bf_custom_genesis_ok.json", &custom_genesis_json());
        let config = base_config(Some(path.clone()));

        let registry = config.with_custom_genesis().unwrap();

        let custom = registry.by_network(&Network::Custom);
        assert_eq!(custom.network_magic as u64, CUSTOM_MAGIC);

        fs::remove_file(path).ok();
    }

    #[test]
    fn with_custom_genesis_toml_is_added() {
        let toml = format!(
            "active_slots_coefficient = 0.05\n\
             update_quorum = 5\n\
             max_lovelace_supply = \"45000000000000000\"\n\
             network_magic = {CUSTOM_MAGIC}\n\
             epoch_length = 432000\n\
             system_start = 1506203091\n\
             slots_per_kes_period = 129600\n\
             slot_length = 1\n\
             max_kes_evolutions = 62\n\
             security_param = 2160\n"
        );
        let path = write_temp("bf_custom_genesis_ok.toml", &toml);
        let config = base_config(Some(path.clone()));

        let registry = config.with_custom_genesis().unwrap();

        assert_eq!(
            registry.by_network(&Network::Custom).network_magic as u64,
            CUSTOM_MAGIC
        );

        fs::remove_file(path).ok();
    }

    #[test]
    fn with_custom_genesis_missing_file_errors() {
        let path = std::env::temp_dir().join("bf_custom_genesis_does_not_exist.json");
        let config = base_config(Some(path));

        let err = config.with_custom_genesis().unwrap_err();
        assert!(
            err.to_string()
                .contains("Failed to read custom genesis file"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn with_custom_genesis_invalid_content_errors() {
        let path = write_temp("bf_custom_genesis_invalid.json", "this is not genesis");
        let config = base_config(Some(path.clone()));

        let err = config.with_custom_genesis().unwrap_err();
        assert!(
            err.to_string()
                .contains("Failed to parse custom genesis file"),
            "unexpected error: {err}"
        );

        fs::remove_file(path).ok();
    }
}
