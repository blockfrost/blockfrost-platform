use crate::types::Network;
use anyhow::{Result, bail};
use clap::Parser;
use serde::{Deserialize, Deserializer};
use std::env::var;
use std::fs::read_to_string;
use std::str::FromStr;
use std::{fs, path::PathBuf};
use tracing::Level;

#[derive(Parser)]
#[command(author,
          version = concat!(env!("CARGO_PKG_VERSION"), " (", env!("GIT_REVISION"), ")"),
          about,
          long_about = None)]
pub struct Args {
    #[arg(short, long, value_name = "FILE")]
    pub config: PathBuf,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerInput {
    pub address: String,
    pub log_level: String,
    pub url: Option<url::Url>,
    /// Base URLs of all gateway peers to advertise in `/register` responses.
    /// Platforms will open a WebSocket connection to each of these for HA.
    /// When empty, the single `url` (or the `Host:` header) is used as fallback.
    #[serde(default)]
    pub peer_urls: Vec<url::Url>,
    /// Shared secret used to derive the 32-byte keyed BLAKE3 MAC key for
    /// stateless tokens that any gateway can verify. Always required (via
    /// `peer_secret` or `peer_secret_file`).
    pub peer_secret: Option<String>,
    pub peer_secret_file: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DbInput {
    pub connection_string: Option<String>,
    pub connection_string_file: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BlockfrostInput {
    pub project_id: Option<String>,
    pub project_id_file: Option<String>,
    pub nft_asset: String,
}

fn deserialize_log_level<'de, D>(deserializer: D) -> Result<Level, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Level::from_str(&s.to_lowercase()).map_err(serde::de::Error::custom)
}

#[derive(Debug, Deserialize, Clone)]
pub struct Server {
    pub address: String,
    #[serde(deserialize_with = "deserialize_log_level")]
    pub log_level: Level,
    pub network: Network,
    pub url: Option<url::Url>,
    /// Base URLs of all gateway peers to advertise for HA (see [`ServerInput::peer_urls`]).
    pub peer_urls: Vec<url::Url>,
    /// Derived 32-byte keyed BLAKE3 MAC key for stateless tokens (see
    /// [`ServerInput::peer_secret`]).
    pub peer_secret: [u8; 32],
}

#[derive(Debug, Deserialize, Clone)]
pub struct Db {
    pub connection_string: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ConfigInput {
    pub server: ServerInput,
    pub database: DbInput,
    pub blockfrost: BlockfrostInput,
    pub hydra_platform: Option<HydraConfig>,
    pub hydra_bridge: Option<HydraConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: Server,
    pub database: Db,
    pub blockfrost: Blockfrost,
    pub hydra_platform: Option<HydraConfig>,
    pub hydra_bridge: Option<HydraConfig>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Blockfrost {
    pub project_id: String,
    pub nft_asset: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct HydraConfig {
    pub cardano_signing_key: PathBuf,
    pub max_concurrent_hydra_nodes: u64,
    /// How much to commit from [`Self::cardano_signing_key`] when starting a new L2 session.
    pub commit_ada: f64,
    /// How much is a single request worth?
    pub lovelace_per_request: u64,
    /// How many requests to bundle for a single microtransaction payment on L2.
    pub requests_per_microtransaction: u64,
    /// How many L2 microtransactions until we flush to L1.
    pub microtransactions_per_fanout: u64,
}

pub fn load_config(path: PathBuf) -> Config {
    let config_file_content = fs::read_to_string(path).expect("Reading config failed");
    let toml_config: ConfigInput =
        toml::from_str(&config_file_content).expect("Config file is invalid");

    let log_level = match toml_config.server.log_level.to_lowercase().as_str() {
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        "trace" => Level::TRACE,
        _ => Level::INFO,
    };

    let connection_string = match toml_config.database.connection_string_file {
        Some(file_path) => read_to_string(file_path)
            .expect("Failed to read connection string file")
            .to_string(),
        None => toml_config
            .database
            .connection_string
            .expect("connection_string or connection_string_file must be provided"),
    };

    let project_id = match toml_config.blockfrost.project_id_file {
        Some(file_path) => read_to_string(file_path)
            .expect("Failed to read project ID file")
            .to_string(),
        None => toml_config
            .blockfrost
            .project_id
            .expect("project_id must be provided"),
    };

    let network = network_from_project_id(&project_id).expect("invalid Blockfrost project_id");

    let peer_urls = toml_config.server.peer_urls;
    for u in &peer_urls {
        validate_server_url(u);
    }

    let peer_secret_raw = match toml_config.server.peer_secret_file {
        Some(file_path) => read_to_string(file_path)
            .expect("Failed to read peer secret file")
            .trim()
            .to_string(),
        None => toml_config
            .server
            .peer_secret
            .expect("peer_secret or peer_secret_file must be provided"),
    };
    let peer_secret = derive_peer_key(&peer_secret_raw);

    let config = Config {
        server: Server {
            address: toml_config.server.address,
            log_level,
            network,
            url: toml_config.server.url.inspect(validate_server_url),
            peer_urls,
            peer_secret,
        },
        database: Db { connection_string },
        blockfrost: Blockfrost {
            project_id,
            nft_asset: toml_config.blockfrost.nft_asset,
        },
        hydra_platform: toml_config.hydra_platform,
        hydra_bridge: toml_config.hydra_bridge,
    };

    override_with_env(config)
}

/// Derive a 32-byte key from an arbitrary-length secret string using Blake3.
fn derive_peer_key(secret: &str) -> [u8; 32] {
    *blake3::hash(secret.as_bytes()).as_bytes()
}

/// Validate that a parsed `server.url` uses http(s) and includes a host.
/// Panics on violation so that misconfiguration is caught at startup.
fn validate_server_url(url: &url::Url) {
    match url.scheme() {
        "http" | "https" => {},
        other => panic!("server.url must use http:// or https://, got: {other}://"),
    }
    assert!(
        url.host().is_some(),
        "server.url must include a host, got: {url}"
    );
}

/// Parse a raw string into a [`url::Url`] and validate it.
/// Used for environment variable overrides of server URLs.
fn parse_server_url(raw: &str, env_var: &str) -> url::Url {
    let parsed = url::Url::parse(raw)
        .unwrap_or_else(|e| panic!("{env_var} is not a valid URL ({raw}): {e}"));
    validate_server_url(&parsed);
    parsed
}

fn network_from_project_id(project_id: &str) -> Result<Network> {
    if project_id.starts_with("mainnet") {
        Ok(Network::Mainnet)
    } else if project_id.starts_with("preprod") {
        Ok(Network::Preprod)
    } else if project_id.starts_with("preview") {
        Ok(Network::Preview)
    } else {
        bail!("Blockfrost project_id must start with 'mainnet', 'preprod', or 'preview'")
    }
}

fn override_with_env(config: Config) -> Config {
    let server_url = var("BLOCKFROST_GATEWAY_SERVER_URL")
        .ok()
        .map(|s| parse_server_url(&s, "BLOCKFROST_GATEWAY_SERVER_URL"))
        .or(config.server.url);
    let peer_urls = var("BLOCKFROST_GATEWAY_SERVER_PEER_URLS")
        .ok()
        .map(|s| {
            s.split(',')
                .map(|u| parse_server_url(u.trim(), "BLOCKFROST_GATEWAY_SERVER_PEER_URLS"))
                .collect()
        })
        .unwrap_or(config.server.peer_urls);
    let peer_secret = var("BLOCKFROST_GATEWAY_SERVER_PEER_SECRET_FILE")
        .ok()
        .map(|path| {
            let raw = read_to_string(path)
                .expect("Failed to read BLOCKFROST_GATEWAY_SERVER_PEER_SECRET_FILE")
                .trim()
                .to_string();
            derive_peer_key(&raw)
        })
        .or_else(|| {
            var("BLOCKFROST_GATEWAY_SERVER_PEER_SECRET")
                .ok()
                .map(|s| derive_peer_key(&s))
        })
        .unwrap_or(config.server.peer_secret);
    let server_address = var("BLOCKFROST_GATEWAY_SERVER_ADDRESS").unwrap_or(config.server.address);
    let log_level_str = var("BLOCKFROST_GATEWAY_SERVER_LOG_LEVEL")
        .unwrap_or_else(|_| config.server.log_level.to_string());
    let db_connection =
        var("BLOCKFROST_GATEWAY_DB_CONNECTION_STRING").unwrap_or(config.database.connection_string);
    let project_id = var("BLOCKFROST_GATEWAY_PROJECT_ID").unwrap_or(config.blockfrost.project_id);
    let nft_asset = var("BLOCKFROST_GATEWAY_NFT_ASSET").unwrap_or(config.blockfrost.nft_asset);
    let network = network_from_project_id(&project_id).expect("invalid Blockfrost project_id");

    let final_log_level = match log_level_str.to_lowercase().as_str() {
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        "trace" => Level::TRACE,
        _ => Level::INFO,
    };

    Config {
        server: Server {
            address: server_address,
            log_level: final_log_level,
            network,
            url: server_url,
            peer_urls,
            peer_secret,
        },
        database: Db {
            connection_string: db_connection,
        },
        blockfrost: Blockfrost {
            project_id,
            nft_asset,
        },
        hydra_platform: config.hydra_platform,
        hydra_bridge: config.hydra_bridge,
    }
}
