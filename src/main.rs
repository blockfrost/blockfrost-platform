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
use dotenvy::dotenv;
use std::sync::Arc;
use tokio::signal;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // Load .env file
    dotenv().ok();

    // CLI
    let arguments = Args::parse();
    let config = Arc::new(Config::from_args(arguments)?);

    // Logging
    setup_tracing(config.log_level);

    // Build app
    let (app, node_conn_pool) = build(config.clone()).await?;

    // Bind server
    let address = format!("{}:{}", config.server_address, config.server_port);
    let listener = tokio::net::TcpListener::bind(&address).await?;

    info!(
        "Server is listening on http://{}:{}/",
        config.server_address, config.server_port
    );

    // Shutdown signal
    let shutdown_signal = async {
        let _ = signal::ctrl_c().await;
        info!("Received shutdown signal");
    };

    // Spawn background tasks
    tokio::spawn(node_health_check_task(node_conn_pool));

    // Serve
    axum::serve(listener, ServiceExt::<Request>::into_make_service(app))
        .with_graceful_shutdown(shutdown_signal)
        .await?;

    Ok(())
}
