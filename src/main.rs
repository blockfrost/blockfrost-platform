use axum::extract::Request;
use axum::ServiceExt;
use blockfrost_platform::{
    background_tasks::node_health_check_task,
    cli::{Args, Config},
    logging::setup_tracing,
    server::build,
    AppError,
};
use clap::Parser;
use std::sync::Arc;
use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // CLI
    let arguments = Args::parse();
    let config = Arc::new(Config::from_args(arguments)?);

    // Logging
    setup_tracing(config.log_level);

    // Build app
    let (app, node_conn_pool, icebreakers_api) = build(config.clone()).await?;

    // Bind server
    let address = format!("{}:{}", config.server_address, config.server_port);
    let listener = tokio::net::TcpListener::bind(&address).await?;

    // Shutdown signal
    let shutdown_signal = async {
        let _ = signal::ctrl_c().await;
        info!("Received shutdown signal");
    };

    // Spawn background tasks
    tokio::spawn(node_health_check_task(node_conn_pool));

    // Create server task
    let spawn_task = tokio::spawn(async move {
        axum::serve(listener, ServiceExt::<Request>::into_make_service(app))
            .with_graceful_shutdown(shutdown_signal)
            .await
    });

    // Register with Icebreakers API after server is up
    if let Some(icebreakers_api) = icebreakers_api {
        icebreakers_api.register().await?;
    }

    info!(
        "Server is listening on http://{}:{}/",
        config.server_address, config.server_port
    );

    // Wait for the server task to finish
    spawn_task
        .await
        .map_err(|err| AppError::Server(err.to_string()))??;

    Ok(())
}
