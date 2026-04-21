use anyhow::Result;
use api::{register, root};
use axum::{
    Extension, Router,
    routing::{get, post},
};
use blockfrost_gateway::{
    api, blockfrost, config, db, hydra_server_bridge, hydra_server_platform, load_balancer,
    rate_limit, sdk_bridge_ws,
};
use clap::Parser;
use colored::Colorize;
use config::{Args, Config};
use db::DB;
use std::net::SocketAddr;
use tracing::Level;
use tracing_subscriber::fmt::format::Format;

fn setup_tracing(log_level: Level) {
    #[cfg(target_os = "linux")]
    if std::env::var("BLOCKFROST_GATEWAY_LOG_TARGET")
        .ok()
        .as_deref()
        == Some("journal")
    {
        if let Ok(journald_layer) = tracing_journald::layer() {
            use tracing_subscriber::prelude::*;
            tracing_subscriber::registry()
                .with(journald_layer)
                .with(tracing_subscriber::filter::LevelFilter::from_level(
                    log_level,
                ))
                .init();
            return;
        }
        eprintln!(
            "WARNING: BLOCKFROST_GATEWAY_LOG_TARGET=journal but failed to connect to journald, falling back to stdout"
        );
    }

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .event_format(
            Format::default()
                .with_ansi(true)
                .with_level(true)
                .with_target(true)
                .compact(),
        )
        .init();
}

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

    dotenvy::dotenv().ok();

    let arguments = Args::parse();
    let config: Config = config::load_config(arguments.config);

    setup_tracing(config.server.log_level);

    let pool = DB::new(&config.database.connection_string).await;
    let blockfrost_api = blockfrost::BlockfrostAPI::new(&config.blockfrost.project_id);
    let hydras_manager = if let Some(hydra_platform_config) = &config.hydra_platform {
        Some(
            hydra_server_platform::HydrasManager::new(
                hydra_platform_config,
                &config.server.network,
                &config.blockfrost.project_id,
            )
            .await?,
        )
    } else {
        None
    };
    let hydras_bridge_manager = if let Some(hydra_bridge_config) = &config.hydra_bridge {
        Some(
            hydra_server_bridge::HydrasManager::new(
                hydra_bridge_config,
                &config.server.network,
                &config.blockfrost.project_id,
            )
            .await?,
        )
    } else {
        None
    };
    let load_balancer = load_balancer::LoadBalancerState::new(hydras_manager).await;
    let register_rate_limiter = rate_limit::new_register_rate_limiter();

    let base_router = Router::new()
        .route("/", get(root::route))
        .route("/register", post(register::route))
        .route("/ws", get(load_balancer::api::websocket_route))
        .route("/stats", get(load_balancer::api::stats_route))
        .route(
            "/{uuid}",
            axum::routing::any(load_balancer::api::prefix_route_root),
        )
        .route(
            "/{uuid}/",
            axum::routing::any(load_balancer::api::prefix_route_root),
        )
        .route(
            "/{uuid}/{*rest}",
            axum::routing::any(load_balancer::api::prefix_route),
        )
        .layer(Extension(load_balancer))
        .layer(Extension(config.clone()))
        .layer(Extension(pool))
        .layer(Extension(blockfrost_api))
        .layer(Extension(register_rate_limiter));

    let sdk_state = sdk_bridge_ws::SdkBridgeState::new(base_router.clone(), hydras_bridge_manager);

    let app = base_router
        .route("/sdk/ws", get(sdk_bridge_ws::websocket_route))
        .layer(Extension(sdk_state));

    let listener = tokio::net::TcpListener::bind(&config.server.address)
        .await
        .expect("Failed to bind to address");

    println!(
        "{}",
        format!(
            "\nAddress: 🌍 http://{}\n\
             Log Level: 📘 {}\n",
            config.server.address, config.server.log_level,
        )
        .white()
        .bold()
    );

    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .await
    .unwrap_or_else(|e| {
        eprintln!("Server error: {e}");
        std::process::exit(1);
    });

    Ok(())
}
