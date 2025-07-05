use common::{config::Config as RootConfig, errors::AppError};
use serde::{Deserialize, Serialize};
use std::{fs, path::Path};

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub upstream: Upstream,
    pub storage: Storage,
    pub genesis: Genesis,
    pub sync: Sync,
    pub submit: Submit,

    #[serde(rename = "serve.grpc")]
    pub serve_grpc: ServeGrpc,

    #[serde(rename = "serve.minibf")]
    pub serve_minibf: ServeMinibf,

    #[serde(rename = "serve.ouroboros")]
    pub serve_ouroboros: ServeOuroboros,

    pub relay: Relay,
    pub mithril: Mithril,
    pub logging: Logging,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Upstream {
    pub peer_address: String,
    pub network_magic: u64,
    pub is_testnet: bool,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Storage {
    pub version: String,
    pub path: String,
    pub max_wal_history: u64,
    pub max_chain_history: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Genesis {
    pub byron_path: String,
    pub shelley_path: String,
    pub alonzo_path: String,
    pub conway_path: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Sync {
    pub pull_batch_size: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Submit {
    pub prune_height: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServeGrpc {
    pub listen_address: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServeMinibf {
    pub listen_address: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct ServeOuroboros {
    pub listen_path: String,
    pub magic: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Relay {
    pub listen_address: String,
    pub magic: u64,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Mithril {
    pub aggregator: String,
    pub genesis_key: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct Logging {
    pub max_level: String,
    pub include_tokio: bool,
    pub include_pallas: bool,
    pub include_grpc: bool,
}

impl Config {
    pub async fn generate_from_root_config(root_config: &RootConfig) -> Result<Self, AppError> {
        Ok(Self {
            upstream: Upstream {
                peer_address: root_config.node_socket_path.clone(),
                network_magic: 1,
                is_testnet: true,
            },
            storage: Storage {
                version: "v1".to_string(),
                path: "data".to_string(),
                max_wal_history: 10_000,
                max_chain_history: 10_000,
            },
            genesis: Genesis {
                byron_path: "byron.json".to_string(),
                shelley_path: "shelley.json".to_string(),
                alonzo_path: "alonzo.json".to_string(),
                conway_path: "conway.json".to_string(),
            },
            sync: Sync {
                pull_batch_size: 100,
            },
            submit: Submit { prune_height: 200 },
            serve_grpc: ServeGrpc {
                listen_address: "[::]:50051".to_string(),
            },
            serve_minibf: ServeMinibf {
                listen_address: "[::]:3000".to_string(),
            },
            serve_ouroboros: ServeOuroboros {
                listen_path: "dolos.socket".to_string(),
                magic: 1,
            },
            relay: Relay {
                listen_address: "[::]:30031".to_string(),
                magic: 1,
            },
            mithril: Mithril {
                aggregator: "https://aggregator.release-preprod.api.mithril.network/aggregator"
                    .to_string(),
                genesis_key: "5b3...45d".to_string(),
            },
            logging: Logging {
                max_level: "INFO".to_string(),
                include_tokio: false,
                include_pallas: false,
                include_grpc: false,
            },
        })
    }

    pub fn save_to_toml<P: AsRef<Path>>(&self, path: P) -> Result<(), AppError> {
        let toml_str = toml::to_string_pretty(&self)
            .map_err(|err| AppError::Dolos(format!("Serialization error: {err}")))?;

        fs::write(path, toml_str)
            .map_err(|err| AppError::Dolos(format!("IO error writing config: {err}")))?;

        Ok(())
    }
}
