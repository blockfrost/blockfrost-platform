use crate::BlockfrostError;
use axum::response::{Extension, IntoResponse};
use metrics::gauge;
use metrics_exporter_prometheus::PrometheusHandle;
use std::sync::Arc;
use tokio::sync::RwLock;

pub async fn route(
    Extension(prometheus_handle): Extension<Arc<RwLock<PrometheusHandle>>>,
    Extension(health_monitor): Extension<crate::health_monitor::HealthMonitor>,
) -> Result<impl IntoResponse, BlockfrostError> {
    gauge!("health_errors_total").set(health_monitor.num_errors().await);

    let handle = prometheus_handle.write().await;
    Ok(handle.render().into_response())
}
