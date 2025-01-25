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

use std::{sync::Arc, time::Duration};
use tokio::{signal, sync::oneshot};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), AppError> {
    // CLI
    let arguments = Args::parse();
    let config = Arc::new(Config::from_args(arguments)?);

    // Logging
    setup_tracing(config.log_level);

    let (app, node_conn_pool, icebreakers_api, api_prefix) = build(config.clone()).await?;
    let address = format!("{}:{}", config.server_address, config.server_port);
    let listener = tokio::net::TcpListener::bind(&address).await?;

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

        info!("waiting 5s");
        tokio::time::sleep(Duration::from_secs(5)).await;
        info!("Waiting done");

        if let Some(icebreakers_api) = &icebreakers_api {
            icebreakers_api.register().await?;
        }

        // Spawn background tasks
        tokio::spawn(node_health_check_task(node_conn_pool));
    }

    spawn_task
        .await
        .map_err(|err| AppError::Server(err.to_string()))??;

    Ok(())
}
