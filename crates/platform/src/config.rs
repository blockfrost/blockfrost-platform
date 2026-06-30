use crate::cli::Args;
use crate::genesis::{GenesisRegistry, GenesisRegistryMut, genesis};
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
    pub genesis: Vec<(Network, GenesisResponse)>,
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

        let custom_genesis = load_custom_genesis(args.custom_genesis_config.as_ref())?;
        let mut genesis_registry = genesis();

        let network = match custom_genesis {
            Some(custom) => {
                genesis_registry.add(Network::Custom, custom);
                Network::Custom
            },
            None => detector(&node_socket_path).await?,
        };

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
            genesis: genesis_registry,
            data_node,
            hydra,
            server_concurrency_limit: args.server_concurrency_limit,
            max_response_body_bytes: args.max_response_body_bytes,
        })
    }

    pub async fn from_args(args: Args) -> Result<Self, AppError> {
        Self::from_args_with_detector(args, |s| detect_network(s).boxed()).await
    }
}

/// Read and parse the optional custom genesis file (JSON or TOML).
///
/// Returns `Ok(None)` when no path is supplied. Returns an error when the file
/// cannot be read or parsed.
fn load_custom_genesis(path: Option<&PathBuf>) -> Result<Option<GenesisResponse>, AppError> {
    let Some(path) = path else {
        return Ok(None);
    };

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

    validate_genesis(&custom).map_err(|e| {
        AppError::Server(format!(
            "Invalid custom genesis file {}: {}",
            path.display(),
            e
        ))
    })?;

    Ok(Some(custom))
}

/// Sanity-check numeric fields that must be non-negative before they get cast
/// to `u64` (e.g. `network_magic` for the node pool) or used as durations and
/// lengths. A negative value would otherwise silently wrap into a huge `u64`
/// and surface much later as a confusing runtime failure.
fn validate_genesis(genesis: &GenesisResponse) -> Result<(), String> {
    let non_negative = [
        ("network_magic", genesis.network_magic),
        ("epoch_length", genesis.epoch_length),
        ("system_start", genesis.system_start),
        ("slots_per_kes_period", genesis.slots_per_kes_period),
        ("slot_length", genesis.slot_length),
        ("max_kes_evolutions", genesis.max_kes_evolutions),
        ("security_param", genesis.security_param),
        ("update_quorum", genesis.update_quorum),
    ];

    for (field, value) in non_negative {
        if value < 0 {
            return Err(format!("`{field}` must be non-negative, got {value}"));
        }
    }

    let asc = genesis.active_slots_coefficient;

    if !(asc > 0.0 && asc <= 1.0) {
        return Err(format!(
            "`active_slots_coefficient` must be in (0, 1], got {asc}"
        ));
    }

    match genesis.max_lovelace_supply.parse::<u64>() {
        Ok(n) if n > 0 => {},
        _ => {
            return Err(format!(
                "`max_lovelace_supply` must be a positive integer, got `{}`",
                genesis.max_lovelace_supply
            ));
        },
    }

    Ok(())
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
    use clap::Parser;
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, Ordering};

    const CUSTOM_GENESIS_JSON: &str = r#"{
        "active_slots_coefficient": 0.1,
        "update_quorum": 7,
        "max_lovelace_supply": "123456789",
        "network_magic": 42,
        "epoch_length": 100,
        "system_start": 1000,
        "slots_per_kes_period": 200,
        "slot_length": 2,
        "max_kes_evolutions": 9,
        "security_param": 11
    }"#;

    const CUSTOM_GENESIS_TOML: &str = r#"
        active_slots_coefficient = 0.1
        update_quorum = 7
        max_lovelace_supply = "123456789"
        network_magic = 42
        epoch_length = 100
        system_start = 1000
        slots_per_kes_period = 200
        slot_length = 2
        max_kes_evolutions = 9
        security_param = 11
    "#;

    /// Writes `contents` to a uniquely named temp file and returns its path.
    fn write_temp(name: &str, contents: &str) -> PathBuf {
        let path = std::env::temp_dir().join(format!("bf_test_{name}"));
        fs::write(&path, contents).expect("failed to write temp genesis file");
        path
    }

    /// Builds solitary `Args` pointed at `socket`, optionally with a custom
    /// genesis file.
    fn args_with(socket: &str, custom_genesis: Option<&PathBuf>) -> Args {
        let mut argv = vec![
            "testing".to_string(),
            "--node-socket-path".to_string(),
            socket.to_string(),
            "--solitary".to_string(),
        ];
        if let Some(path) = custom_genesis {
            argv.push("--custom-genesis-config".to_string());
            argv.push(path.display().to_string());
        }
        Args::try_parse_from(argv).expect("args should parse")
    }

    /// A detector that records whether it was invoked and always reports
    /// `Preview`.
    fn recording_detector(
        called: Arc<AtomicBool>,
    ) -> impl for<'a> Fn(&'a str) -> BoxFuture<'a, Result<Network, AppError>> {
        move |_socket| {
            let called = called.clone();
            async move {
                called.store(true, Ordering::SeqCst);
                Ok(Network::Preview)
            }
            .boxed()
        }
    }

    #[tokio::test]
    async fn without_custom_genesis_uses_detector_and_builtin_registry() {
        let called = Arc::new(AtomicBool::new(false));
        let args = args_with("/path/to/socket", None);

        let config = Config::from_args_with_detector(args, recording_detector(called.clone()))
            .await
            .expect("config should build");

        assert!(called.load(Ordering::SeqCst), "detector must be called");
        assert_eq!(config.network, Network::Preview);
        // Registry is the built-in one, untouched.
        assert_eq!(config.genesis, genesis());
        assert_eq!(config.genesis.len(), 3);
    }

    #[tokio::test]
    async fn custom_genesis_json_sets_custom_network_and_merges_registry() {
        let path = write_temp("genesis_json.json", CUSTOM_GENESIS_JSON);
        let called = Arc::new(AtomicBool::new(false));
        let args = args_with("/path/to/socket", Some(&path));

        let config = Config::from_args_with_detector(args, recording_detector(called.clone()))
            .await
            .expect("config should build");

        // The detector must be skipped entirely when a custom genesis is given.
        assert!(
            !called.load(Ordering::SeqCst),
            "detector must NOT be called for custom genesis"
        );
        assert_eq!(config.network, Network::Custom);

        // The custom entry is reachable by the network the server actually uses.
        let custom = config.genesis.by_network(&Network::Custom);
        assert_eq!(custom.network_magic, 42);
        assert_eq!(custom.epoch_length, 100);
        assert_eq!(custom.security_param, 11);

        // It is prepended, and the built-in networks are preserved alongside it.
        assert_eq!(config.genesis[0].0, Network::Custom);
        assert_eq!(config.genesis.len(), 4);
        assert_eq!(
            config.genesis.by_network(&Network::Mainnet).network_magic,
            764_824_073
        );

        // The custom magic is now detectable via the merged registry.
        assert_eq!(config.genesis.by_magic(42).network_magic, 42);
        assert_eq!(config.genesis.network_by_magic(42), &Network::Custom);

        let _ = fs::remove_file(&path);
    }

    #[tokio::test]
    async fn custom_genesis_accepts_toml() {
        let path = write_temp("genesis_toml.toml", CUSTOM_GENESIS_TOML);
        let args = args_with("/path/to/socket", Some(&path));

        let config = Config::from_args_with_detector(
            args,
            recording_detector(Arc::new(AtomicBool::new(false))),
        )
        .await
        .expect("config should build from TOML genesis");

        assert_eq!(config.network, Network::Custom);
        assert_eq!(
            config.genesis.by_network(&Network::Custom).network_magic,
            42
        );

        let _ = fs::remove_file(&path);
    }

    #[tokio::test]
    async fn custom_genesis_missing_file_errors() {
        let missing = std::env::temp_dir().join("bf_test_does_not_exist_genesis.json");
        let _ = fs::remove_file(&missing);
        let args = args_with("/path/to/socket", Some(&missing));

        let err = Config::from_args_with_detector(
            args,
            recording_detector(Arc::new(AtomicBool::new(false))),
        )
        .await
        .expect_err("missing genesis file must error");

        assert!(format!("{err:?}").contains("Failed to read custom genesis file"));
    }

    #[tokio::test]
    async fn custom_genesis_invalid_contents_errors() {
        let path = write_temp("genesis_invalid.json", "this is neither json nor toml: {[");
        let args = args_with("/path/to/socket", Some(&path));

        let err = Config::from_args_with_detector(
            args,
            recording_detector(Arc::new(AtomicBool::new(false))),
        )
        .await
        .expect_err("invalid genesis file must error");

        assert!(format!("{err:?}").contains("Failed to parse custom genesis file"));

        let _ = fs::remove_file(&path);
    }

    #[tokio::test]
    async fn custom_genesis_rejects_negative_network_magic() {
        // A negative magic would otherwise wrap into a huge u64 when cast.
        let bad = CUSTOM_GENESIS_JSON.replace("\"network_magic\": 42", "\"network_magic\": -1");
        let path = write_temp("genesis_negative_magic.json", &bad);
        let args = args_with("/path/to/socket", Some(&path));

        let err = Config::from_args_with_detector(
            args,
            recording_detector(Arc::new(AtomicBool::new(false))),
        )
        .await
        .expect_err("negative network_magic must error");

        let msg = format!("{err:?}");
        assert!(msg.contains("Invalid custom genesis file"), "got: {msg}");
        assert!(msg.contains("network_magic"), "got: {msg}");

        let _ = fs::remove_file(&path);
    }

    #[tokio::test]
    async fn custom_genesis_rejects_out_of_range_active_slots_coefficient() {
        for (name, bad_value) in [("zero", "0.0"), ("above_one", "1.5")] {
            let bad = CUSTOM_GENESIS_JSON.replace(
                "\"active_slots_coefficient\": 0.1",
                &format!("\"active_slots_coefficient\": {bad_value}"),
            );
            let path = write_temp(&format!("genesis_bad_asc_{name}.json"), &bad);
            let args = args_with("/path/to/socket", Some(&path));

            let err = Config::from_args_with_detector(
                args,
                recording_detector(Arc::new(AtomicBool::new(false))),
            )
            .await
            .expect_err("out-of-range active_slots_coefficient must error");

            let msg = format!("{err:?}");
            assert!(msg.contains("Invalid custom genesis file"), "got: {msg}");
            assert!(msg.contains("active_slots_coefficient"), "got: {msg}");

            let _ = fs::remove_file(&path);
        }
    }

    #[tokio::test]
    async fn custom_genesis_rejects_non_positive_max_lovelace_supply() {
        for (name, bad_value) in [("zero", "0"), ("garbage", "not-a-number")] {
            let bad = CUSTOM_GENESIS_JSON.replace(
                "\"max_lovelace_supply\": \"123456789\"",
                &format!("\"max_lovelace_supply\": \"{bad_value}\""),
            );
            let path = write_temp(&format!("genesis_bad_supply_{name}.json"), &bad);
            let args = args_with("/path/to/socket", Some(&path));

            let err = Config::from_args_with_detector(
                args,
                recording_detector(Arc::new(AtomicBool::new(false))),
            )
            .await
            .expect_err("non-positive max_lovelace_supply must error");

            let msg = format!("{err:?}");
            assert!(msg.contains("Invalid custom genesis file"), "got: {msg}");
            assert!(msg.contains("max_lovelace_supply"), "got: {msg}");

            let _ = fs::remove_file(&path);
        }
    }
}
