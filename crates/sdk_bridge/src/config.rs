use crate::types::Network;
use anyhow::{Result, anyhow};
use clap::Parser;
use std::net::SocketAddr;
use std::path::PathBuf;
use url::Url;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    #[arg(
        long,
        default_value = "wss://icebreakers1.platform.blockfrost.io/sdk/ws"
    )]
    pub gateway_ws_url: String,

    #[arg(long, default_value = "127.0.0.1:3002")]
    pub listen_address: String,

    #[arg(long, value_enum)]
    pub network: Network,

    #[arg(long, value_name = "FILE")]
    pub node_socket_path: PathBuf,

    #[arg(long, value_name = "FILE")]
    pub cardano_signing_key: PathBuf,
}

#[derive(Clone, Debug)]
pub struct BridgeConfig {
    pub gateway_ws_url: String,
    pub listen_address: SocketAddr,
    pub network: Network,
    pub node_socket_path: PathBuf,
    pub cardano_signing_key: PathBuf,
}

impl BridgeConfig {
    pub fn from_args(args: Args) -> Result<Self> {
        let listen_address = args
            .listen_address
            .parse::<SocketAddr>()
            .map_err(|err| anyhow!("Invalid listen address: {err}"))?;
        let gateway_ws_url = normalize_gateway_ws_url(&args.gateway_ws_url)?;

        Ok(Self {
            gateway_ws_url,
            listen_address,
            network: args.network,
            node_socket_path: args.node_socket_path,
            cardano_signing_key: args.cardano_signing_key,
        })
    }
}

fn normalize_gateway_ws_url(raw: &str) -> Result<String> {
    let mut url = Url::parse(raw).map_err(|err| anyhow!("Invalid gateway URL: {err}"))?;

    match url.scheme() {
        "http" => {
            url.set_scheme("ws")
                .map_err(|_| anyhow!("invalid URL scheme"))?;
        },
        "https" => {
            url.set_scheme("wss")
                .map_err(|_| anyhow!("invalid URL scheme"))?;
        },
        "ws" | "wss" => {},
        other => {
            return Err(anyhow!("Unsupported URL scheme: {other}"));
        },
    }

    let path = url.path().to_string();
    if path.is_empty() || path == "/" {
        url.set_path("/sdk/ws");
    } else if path.ends_with("/sdk/ws/") {
        let trimmed = path.trim_end_matches('/');
        url.set_path(trimmed);
    } else if !path.ends_with("/sdk/ws") {
        let new_path = if path.ends_with('/') {
            format!("{path}sdk/ws")
        } else {
            format!("{path}/sdk/ws")
        };
        url.set_path(&new_path);
    }

    Ok(url.to_string())
}
