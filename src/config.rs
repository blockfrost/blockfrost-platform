use clap::Parser;
use serde::{Deserialize, Deserializer};
use std::env::var;
use std::str::FromStr;
use std::{fs, path::PathBuf};
use tracing::Level;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(short, long, value_name = "FILE")]
    pub config: PathBuf,

    #[arg(short, long, action = clap::ArgAction::Count)]
    debug: u8,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerInput {
    pub address: String,
    pub log_level: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DbInput {
    pub connection_string: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BlockfrostInput {
    pub project_id: String,
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
}

#[derive(Debug, Deserialize, Clone)]
pub struct Config {
    pub server: Server,
    pub database: Db,
    pub blockfrost: Blockfrost,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Blockfrost {
    pub project_id: String,
    pub nft_asset: String,
}

pub fn load_config(path: PathBuf) -> Config {
    let config_file_content = fs::read_to_string(path).expect("Reading config failed");
    let toml_config: ConfigInput = toml::from_str(&config_file_content).expect("Config file is invalid");

    let log_level = match toml_config.server.log_level.to_lowercase().as_str() {
        "debug" => Level::DEBUG,
        "info" => Level::INFO,
        "warn" => Level::WARN,
        "error" => Level::ERROR,
        "trace" => Level::TRACE,
        _ => Level::INFO,
    };

    let config = Config {
        server: Server {
            address: toml_config.server.address,
            log_level,
        },
        database: Db {
            connection_string: toml_config.database.connection_string,
        },
        blockfrost: Blockfrost {
            project_id: toml_config.blockfrost.project_id,
            nft_asset: toml_config.blockfrost.nft_asset,
        },
    };

    override_with_env(config)
}

fn override_with_env(config: Config) -> Config {
    let server_address = var("SERVER_ADDRESS").unwrap_or(config.server.address);
    let log_level_str = var("SERVER_LOG_LEVEL").unwrap_or_else(|_| config.server.log_level.to_string());
    let db_connection = var("DB_CONNECTION_STRING").unwrap_or(config.database.connection_string);
    let project_id = var("BLOCKFROST_PROJECT_ID").unwrap_or(config.blockfrost.project_id);
    let nft_asset = var("BLOCKFROST_NFT_ASSET").unwrap_or(config.blockfrost.nft_asset);

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
        },
        database: Db {
            connection_string: db_connection,
        },
        blockfrost: Blockfrost { project_id, nft_asset },
    }
}
