use crate::errors::BlockfrostError;
use axum::response::{Extension, IntoResponse};
use metrics::{describe_counter, describe_gauge, gauge};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn route(
    Extension(prometheus_handle): Extension<Arc<RwLock<PrometheusHandle>>>,
) -> Result<impl IntoResponse, BlockfrostError> {
    let handle = prometheus_handle.write().await;

    Ok(handle.render().into_response())
}

pub fn setup_metrics_recorder() -> Arc<RwLock<PrometheusHandle>> {
    let builder = PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install Prometheus recorder");

    describe_counter!(
        "http_requests_total",
        "HTTP calls made to blockfrost-platform API"
    );

    describe_gauge!(
        "cardano_node_connections",
        "Number of currently open Cardano node N2C connections"
    );

    // Otherwise it’s not present under `GET /metrics` if we start with a failing cardano-node:
    gauge!("cardano_node_connections").set(0);

    Arc::new(RwLock::new(builder))
}
