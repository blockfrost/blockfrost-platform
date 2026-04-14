mod config;
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
    // Fail early if hydra-node is not found (not applicable on Windows).
    #[cfg(not(target_os = "windows"))]
    if let Err(e) =
        bf_common::find_libexec::find_libexec("hydra-node", "HYDRA_NODE_PATH", &["--version"])
    {
        eprintln!("Error: {e}");
        std::process::exit(1);
    }

    let args = config::Args::parse();
    let config = config::BridgeConfig::from_args(args)?;

    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .event_format(
            Format::default()
                .with_ansi(true)
                .with_level(true)
                .with_target(true)
                .compact(),
        )
        .init();

    let hydra_config = hydra_client::HydraConfig {
        cardano_signing_key: config.cardano_signing_key.clone(),
        blockfrost_project_id: config.blockfrost_project_id.clone(),
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
