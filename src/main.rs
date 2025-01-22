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
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), AppError> {
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

    // 5. Spawn background tasks
    tokio::spawn(node_health_check_task(node_conn_pool));

    // 6. Serve
    axum::serve(listener, ServiceExt::<Request>::into_make_service(app)).await?;

    Ok(())
}
