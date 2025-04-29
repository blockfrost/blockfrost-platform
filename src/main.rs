#![warn(
    clippy::all,
    // clippy::restriction,
    clippy::pedantic,
    clippy::nursery,
    // clippy::cargo
)]

use blockfrost_platform::{
    cli::Args, errors::AppError, load_balancer, logging::setup_tracing, server::build,
};
use dotenvy::dotenv;
use std::sync::Arc;
use tokio::sync::Mutex;
use tracing::{error, info, warn};

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenv().ok();
    let config = Args::init()?;

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
    if let Some(icebreakers_api) = icebreakers_api {
        let health_errors = Arc::new(Mutex::new(vec![]));
        health_monitor
            .register_error_source(health_errors.clone())
            .await;

        tokio::spawn(async move {
            'load_balancers: loop {
                match icebreakers_api.register().await {
                    Ok(response) => {
                        let configs: Vec<_> =
                            response.load_balancers.into_iter().flatten().collect();
                        if configs.is_empty() {
                            warn!("IceBreakers: no WebSocket load balancers to connect to");
                            // If there are no load balancers, only register once, nothing to monitor:
                            break 'load_balancers;
                        }

                        load_balancer::run_all(
                            configs,
                            app.clone(),
                            health_errors.clone(),
                            api_prefix.clone(),
                        )
                        .await;

                        let delay = std::time::Duration::from_secs(1);
                        info!("IceBreakers: will re-register in {:?}", delay);
                        tokio::time::sleep(delay).await;
                    },
                    Err(err) => {
                        let delay = std::time::Duration::from_secs(10);
                        error!(
                            "IceBreakers registration failed: {}, will re-register in {:?}",
                            err, delay
                        );
                        *health_errors.lock().await = vec![err.into()];
                        tokio::time::sleep(delay).await;
                    },
                };
            }
        });
    }

    spawn_task
        .await
        .map_err(|err| AppError::Server(err.to_string()))??;

    Ok(())
}
