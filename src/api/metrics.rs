use crate::BlockfrostError;
use axum::response::{Extension, IntoResponse};
use metrics::{describe_counter, describe_gauge, gauge};
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn route(
    Extension(prometheus_handle): Extension<Option<Arc<RwLock<PrometheusHandle>>>>,
) -> Result<impl IntoResponse, BlockfrostError> {
    match prometheus_handle {
        None => Err(BlockfrostError::not_found()),
        Some(handle) => {
            let handle = handle.write().await;
            Ok(handle.render().into_response())
        }
    }
}

pub fn setup_metrics_recorder() -> Arc<RwLock<PrometheusHandle>> {
    let builder = PrometheusBuilder::new()
        .install_recorder()
        .expect("failed to install Prometheus recorder");

    // Note: we’re initializing the gauges with 0, otherwise they’re not present
    // under `GET /metrics` right after startup, before anything happens.

    describe_counter!(
        "http_requests_total",
        "HTTP calls made to blockfrost-platform API"
    );

    describe_gauge!(
        "cardano_node_connections",
        "Number of currently open Cardano node N2C connections"
    );
    gauge!("cardano_node_connections").set(0);

    describe_gauge!(
        "cardano_node_connections_initiated",
        "Number of Cardano node N2C connections that have ever been initiated"
    );
    gauge!("cardano_node_connections_initiated").set(0);

    describe_gauge!(
        "cardano_node_connections_failed",
        "Number of Cardano node N2C connections that failed and had to be restarted"
    );
    gauge!("cardano_node_connections_failed").set(0);

    describe_gauge!(
        "tx_submit_success",
        "Number of transactions that were successfully submitted"
    );
    gauge!("tx_submit_success").set(0);

    describe_gauge!(
        "tx_submit_failure",
        "Number of transactions that were submitted with an error"
    );
    gauge!("tx_submit_failure").set(0);

    Arc::new(RwLock::new(builder))
}
