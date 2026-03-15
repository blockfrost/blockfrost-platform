mod common;

mod tests {
    use blockfrost_platform::BlockfrostError;
    use std::net::{IpAddr, SocketAddr};
    use std::sync::Arc;
    use tokio::sync::Mutex;

    use crate::common::{build_app_non_solitary, initialize_logging};
    use axum::ServiceExt as AxumServiceExt;
    use axum::extract::Request as AxumExtractRequest;
    use blockfrost_platform::icebreakers::manager::IcebreakersManager;
    use tokio::sync::oneshot;
    use tracing::info;

    // Test: `icebreakers register` success registration
    #[tokio::test]
    #[ntest::timeout(120_000)]
    async fn test_icebreakers_registrations() -> Result<(), BlockfrostError> {
        initialize_logging();

        let (app, _, _, icebreakers_api, api_prefix) = build_app_non_solitary()
            .await
            .expect("Failed to build the application");

        let ip_addr: IpAddr = "0.0.0.0".parse().unwrap();
        let address = SocketAddr::new(ip_addr, 3000);
        let listener = tokio::net::TcpListener::bind(address).await.unwrap();
        let (shutdown_tx, shutdown_rx) = oneshot::channel::<()>();
        let (ready_tx, ready_rx) = oneshot::channel();

        let spawn_task = tokio::spawn({
            let app = app.clone();

            async move {
                let server_future = axum::serve(
                    listener,
                    AxumServiceExt::<AxumExtractRequest>::into_make_service(app),
                )
                .with_graceful_shutdown(async {
                    shutdown_rx.await.ok();
                });

                let _ = ready_tx.send(());
                server_future.await
            }
        });

        if ready_rx.await.is_ok() {
            info!("Server is listening on http://{}{}", address, api_prefix);

            if let Some(icebreakers_api) = icebreakers_api {
                let health_errors = Arc::new(Mutex::new(vec![]));

                let manager = IcebreakersManager::new(
                    icebreakers_api.clone(),
                    health_errors.clone(),
                    app.clone(),
                    api_prefix.clone(),
                );

                let response = manager.run_once().await?;
                let resp = response;
                let errors = health_errors.lock().await;

                info!("run_once response: {}", resp);

                assert!(
                    errors.is_empty(),
                    "Expected no WebSocket errors, but found: {:?}",
                    *errors
                );

                assert!(
                    resp.contains("Started"),
                    "Expected successful registration, but got: {resp}",
                );

                tokio::spawn(async move {
                    use tokio::sync::mpsc;
                    let (_, kex_req_rx) = mpsc::channel(32);
                    let (kex_resp_tx, _) = mpsc::channel(32);
                    let (terminate_req_tx, _) = mpsc::channel(32);
                    manager
                        .run((kex_req_rx, kex_resp_tx, terminate_req_tx))
                        .await;
                });
            }
        }

        let _ = shutdown_tx.send(());

        spawn_task.await.unwrap().unwrap();

        Ok(())
    }
}
