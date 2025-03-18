use axum::{ServiceExt, extract::Request};
use blockfrost_platform::{cli::Args, errors::AppError, logging::setup_tracing, server::build};
use dotenvy::dotenv;
use tokio::{signal, sync::oneshot};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    dotenv().ok();
    let config = Args::init().unwrap_or_else(|e| {
        eprintln!("\n{}", e);
        std::process::exit(1);
    });

    // Logging
    setup_tracing(config.log_level);

    info!(
        "Starting {} {} ({})",
        env!("CARGO_PKG_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("GIT_REVISION")
    );

    let (app, _, icebreakers_api, api_prefix) = build(config.clone().into()).await?;
    let address = std::net::SocketAddr::new(config.server_address, config.server_port);
    let listener = tokio::net::TcpListener::bind(address).await?;
    let (ready_tx, ready_rx) = oneshot::channel();
    let shutdown_signal = async {
        let _ = signal::ctrl_c().await;
        info!("Received shutdown signal");
    };

    // Spawn the server in its own task
    let spawn_task = tokio::spawn({
        let app = app;
        async move {
            let server_future =
                axum::serve(listener, ServiceExt::<Request>::into_make_service(app))
                    .with_graceful_shutdown(shutdown_signal);

            // Notify that the server has reached the listening stage
            let _ = ready_tx.send(());

            server_future.await
        }
    });

    if let Ok(()) = ready_rx.await {
        info!("Server is listening on http://{}{}", address, api_prefix);

        if let Some(icebreakers_api) = &icebreakers_api {
            icebreakers_api.register().await?;
        }
    }

    spawn_task
        .await
        .map_err(|err| AppError::Server(err.to_string()))??;

    Ok(())
}
