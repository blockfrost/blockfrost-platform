mod config;
mod find_libexec;
mod http_proxy;
mod hydra_client;
mod protocol;
mod types;
mod ws_client;

use anyhow::Result;
use clap::Parser;
use tracing::info;
use tracing_subscriber::fmt::format::Format;

#[tokio::main]
async fn main() -> Result<()> {
    let args = config::Args::parse();
    let config = config::BridgeConfig::from_args(args)?;

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .event_format(
            Format::default()
                .with_ansi(true)
                .with_level(true)
                .with_target(false)
                .compact(),
        )
        .init();

    let hydra_config = hydra_client::HydraConfig {
        cardano_signing_key: config.cardano_signing_key.clone(),
        node_socket_path: config.node_socket_path.clone(),
        network: config.network.clone(),
    };

    let bridge = ws_client::start(ws_client::BridgeWsConfig {
        ws_url: config.gateway_ws_url.clone(),
        hydra: hydra_config,
    })
    .await?;

    info!(
        "sdk-bridge: proxying HTTP on {} -> {}",
        config.listen_address, config.gateway_ws_url
    );

    http_proxy::serve(config.listen_address, bridge).await?;

    Ok(())
}
