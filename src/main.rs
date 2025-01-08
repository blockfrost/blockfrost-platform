use axum::extract::Request;
use axum::ServiceExt;
use blockfrost_platform::background_tasks::node_health_check_task;
use blockfrost_platform::cli::{Args, Config};
use blockfrost_platform::logging::setup_tracing;
use blockfrost_platform::server::build;
use blockfrost_platform::AppError;
use clap::Parser;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // 1. Parse CLI
    let arguments = Args::parse();
    let config = Config::from_args(arguments)?;

    // 2. Logging
    setup_tracing(&config);

    // 3. Build app
    let (app, node_conn_pool) = build(&config).await?;

    // 4. Bind server
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
