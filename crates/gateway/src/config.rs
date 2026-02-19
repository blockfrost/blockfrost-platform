use crate::types::Network;
use anyhow::{Result, anyhow};
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
    pub url: Option<String>,
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
    pub url: Option<String>,
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
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: Server,
    pub database: Db,
    pub blockfrost: Blockfrost,
    pub hydra_platform: Option<HydraConfig>,
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
    pub node_socket_path: PathBuf,
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
            .expect("connection_string must be provided"),
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

    let network = network_from_project_id(&project_id).unwrap();

    let config = Config {
        server: Server {
            address: toml_config.server.address,
            log_level,
            network,
            url: toml_config.server.url,
        },
        database: Db { connection_string },
        blockfrost: Blockfrost {
            project_id,
            nft_asset: toml_config.blockfrost.nft_asset,
        },
        hydra_platform: toml_config.hydra_platform,
    };

    override_with_env(config)
}

fn network_from_project_id(project_id: &str) -> Result<Network> {
    if project_id.starts_with("mainnet") {
        Ok(Network::Mainnet)
    } else if project_id.starts_with("preprod") {
        Ok(Network::Preprod)
    } else if project_id.starts_with("preview") {
        Ok(Network::Preview)
    } else {
        Err(anyhow!(
            "cannot infer Cardano network from the Blockfrost project id"
        ))
    }
}

fn override_with_env(config: Config) -> Config {
    let server_url = var("SERVER_URL").ok().or(config.server.url.clone());
    let server_address = var("SERVER_ADDRESS").unwrap_or(config.server.address);
    let log_level_str =
        var("SERVER_LOG_LEVEL").unwrap_or_else(|_| config.server.log_level.to_string());
    let db_connection = var("DB_CONNECTION_STRING").unwrap_or(config.database.connection_string);
    let project_id = var("BLOCKFROST_PROJECT_ID").unwrap_or(config.blockfrost.project_id);
    let nft_asset = var("BLOCKFROST_NFT_ASSET").unwrap_or(config.blockfrost.nft_asset);
    let network = network_from_project_id(&project_id).unwrap();

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
        },
        database: Db {
            connection_string: db_connection,
        },
        blockfrost: Blockfrost {
            project_id,
            nft_asset,
        },
        hydra_platform: config.hydra_platform,
    }
}
