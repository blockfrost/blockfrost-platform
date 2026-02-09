#![warn(clippy::all, clippy::pedantic, clippy::nursery)]

use bf_common::cli::Args;
use blockfrost_platform::{
    AppError,
    hydra::client::HydraController,
    icebreakers::manager::IcebreakersManager,
    server::{build, logging::setup_tracing},
};
use dotenvy::dotenv;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc};
use tracing::{info, warn};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenv().ok();
    let config = Args::init().await?;

    // Logging
    setup_tracing(config.log_level);

    info!(
        "Starting {} {} ({})",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("GIT_REVISION")
    );

    let (app, _, health_monitor, icebreakers_api, api_prefix) =
        build(config.clone().into()).await?;

    let address = std::net::SocketAddr::new(config.server_address, config.server_port);
    let listener = tokio::net::TcpListener::bind(address).await?;
    let shutdown_signal = async {
        let _ = tokio::signal::ctrl_c().await;
        info!("Received shutdown signal");
    };

    let notify_server_ready = Arc::new(tokio::sync::Notify::new());

    // Spawn the server in its own task
    let spawn_task = tokio::spawn({
        let notify_server_ready = notify_server_ready.clone();
        let app = app.clone();
        async move {
            let server_future = axum::serve(listener, app.into_make_service())
                .with_graceful_shutdown(shutdown_signal);

            // Notify that the server has reached the listening stage
            notify_server_ready.notify_one();

            server_future.await
        }
    });

    notify_server_ready.notified().await;

    info!("Server is listening on http://{}{}", address, api_prefix);

    // IceBreakers registration and the load balancer task.
    //
    // Whenever a single load balancer connection breaks, we drop all of them,
    // and re-register to get a new set of access tokens. It’s complicated by
    // our want to to specify _multiple_ load balancer endpoints in the future,
    // so it’s best to have future-compatibility in the messaging now.
    let (kex_req_tx, kex_req_rx) = mpsc::channel(32);
    let (kex_resp_tx, kex_resp_rx) = mpsc::channel(32);
    let (terminate_req_tx, terminate_req_rx) = mpsc::channel(32);

    if let Some(icebreakers_api) = icebreakers_api {
        let health_errors = Arc::new(Mutex::new(vec![]));

        health_monitor
            .register_error_source(health_errors.clone())
            .await;

        let manager = IcebreakersManager::new(icebreakers_api, health_errors, app, api_prefix);

        manager
            .run((kex_req_rx, kex_resp_tx, terminate_req_tx))
            .await;
    }

    if let Some(hydra_config) = config.hydra {
        if let Some(icebreakers_config) = config.icebreakers_config {
            let health_errors = Arc::new(Mutex::new(vec![]));
            health_monitor
                .register_error_source(health_errors.clone())
                .await;

            let _controller = HydraController::spawn(
                hydra_config,
                config.network,
                config.node_socket_path,
                icebreakers_config.reward_address,
                health_errors,
                kex_req_tx,
                kex_resp_rx,
                terminate_req_rx,
            )
            .await?;
        } else {
            warn!("Hydra micropayments won’t run without a valid IceBreakers config.");
        }
    }

    spawn_task
        .await
        .map_err(|err| AppError::Server(err.to_string()))??;

    Ok(())
}
