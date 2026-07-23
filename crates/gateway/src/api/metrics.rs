use crate::db::{DB, PoolStatus};
use crate::health_monitor::HealthMonitor;
use crate::load_balancer::LoadBalancerState;
use axum::{Extension, http::StatusCode, response::IntoResponse};
use metrics::{describe_counter, describe_gauge, gauge};
use metrics_exporter_prometheus::formatting::sanitize_label_value;
use metrics_exporter_prometheus::{PrometheusBuilder, PrometheusHandle};
use std::fmt::Write as _;
use std::sync::OnceLock;
use std::sync::atomic;
use tracing::error;
use uuid::Uuid;

static HANDLE: OnceLock<PrometheusHandle> = OnceLock::new();

pub fn setup_metrics_recorder() -> PrometheusHandle {
    HANDLE
        .get_or_init(|| {
            let handle = PrometheusBuilder::new()
                .install_recorder()
                .expect("failed to install Prometheus recorder");

            describe_counter!(
                "blockfrost_gateway_http_requests_total",
                "HTTP requests handled by the Gateway API, by method, route template, and status code."
            );

            describe_gauge!(
                "blockfrost_gateway_build_info",
                "Version and git revision of the running Gateway (always 1)."
            );
            gauge!(
                "blockfrost_gateway_build_info",
                "version" => env!("CARGO_PKG_VERSION"),
                "revision" => env!("GIT_REVISION"),
            )
            .set(1);

            handle
        })
        .clone()
}

struct RelayMetrics {
    relay: String,
    api_prefix: Uuid,
    network_rtt_seconds: Option<f64>,
    connected_since_seconds: i64,
    requests_sent: u64,
    responses_received: u64,
    requests_in_progress: u64,
    healthy: Option<bool>,
    has_data_node: Option<bool>,
    version: Option<String>,
}

fn relay_labels(r: &RelayMetrics) -> String {
    format!(
        "{{relay=\"{}\",api_prefix=\"{}\"}}",
        sanitize_label_value(&r.relay),
        r.api_prefix
    )
}

type RelayMetric = (
    &'static str,
    &'static str,
    &'static str,
    fn(&RelayMetrics) -> Option<String>,
);

const RELAY_METRICS: &[RelayMetric] = &[
    (
        "blockfrost_gateway_relay_up",
        "gauge",
        "Whether the relay is currently connected (always 1).",
        |_| Some("1".to_string()),
    ),
    (
        "blockfrost_gateway_relay_healthy",
        "gauge",
        "Whether the relay’s Platform reported healthy on the last periodic check (absent until the first check completes).",
        |r| r.healthy.map(|h| u8::from(h).to_string()),
    ),
    (
        "blockfrost_gateway_relay_data_node_up",
        "gauge",
        "Whether the relay’s Platform had a data node connected on the last periodic check.",
        |r| r.has_data_node.map(|h| u8::from(h).to_string()),
    ),
    (
        "blockfrost_gateway_relay_network_rtt_seconds",
        "gauge",
        "Last measured WebSocket round-trip time to the relay, in seconds.",
        |r| r.network_rtt_seconds.map(|v| v.to_string()),
    ),
    (
        "blockfrost_gateway_relay_connected_since_seconds",
        "gauge",
        "Unix timestamp (seconds) when the relay connected.",
        |r| Some(r.connected_since_seconds.to_string()),
    ),
    (
        "blockfrost_gateway_relay_requests_sent_total",
        "counter",
        "Total requests forwarded to the relay since it connected, including the Gateway’s periodic health checks.",
        |r| Some(r.requests_sent.to_string()),
    ),
    (
        "blockfrost_gateway_relay_responses_received_total",
        "counter",
        "Total responses received from the relay since it connected, including responses to the Gateway’s periodic health checks.",
        |r| Some(r.responses_received.to_string()),
    ),
    (
        "blockfrost_gateway_relay_requests_in_progress",
        "gauge",
        "Requests currently in flight to the relay.",
        |r| Some(r.requests_in_progress.to_string()),
    ),
];

pub(crate) async fn render_prometheus(
    load_balancer: &LoadBalancerState,
    db_pool: &PoolStatus,
    healthy: bool,
) -> Result<String, std::fmt::Error> {
    let now_chrono = chrono::Utc::now();
    let now_instant = std::time::Instant::now();

    let snapshot: Vec<(Uuid, crate::load_balancer::RelayState)> = load_balancer
        .active_relays
        .lock()
        .await
        .iter()
        .map(|(api_prefix, relay_state)| (*api_prefix, relay_state.clone()))
        .collect();

    let mut relays: Vec<RelayMetrics> = Vec::with_capacity(snapshot.len());
    for (api_prefix, relay_state) in &snapshot {
        let platform_health = relay_state.platform_health.lock().await.clone();
        relays.push(RelayMetrics {
            relay: relay_state.name.0.clone(),
            api_prefix: *api_prefix,
            network_rtt_seconds: relay_state
                .network_rtt
                .lock()
                .await
                .map(|d| d.as_secs_f64()),
            connected_since_seconds: (now_chrono - (now_instant - relay_state.connected_since))
                .timestamp(),
            requests_sent: relay_state.requests_sent.load(atomic::Ordering::SeqCst),
            responses_received: relay_state
                .responses_received
                .load(atomic::Ordering::SeqCst),
            requests_in_progress: relay_state.requests_in_progress.lock().await.len() as u64,
            healthy: platform_health.as_ref().map(|h| h.healthy),
            has_data_node: platform_health.as_ref().and_then(|h| h.has_data_node),
            version: platform_health.and_then(|h| h.version),
        });
    }

    let mut out = String::new();

    writeln!(
        out,
        "# HELP blockfrost_gateway_healthy Whether the Gateway is healthy, i.e. its periodic health checks (PostgreSQL connectivity, Blockfrost API) succeeded. Mirrors the `healthy` field of `GET /`."
    )?;
    writeln!(out, "# TYPE blockfrost_gateway_healthy gauge")?;
    writeln!(out, "blockfrost_gateway_healthy {}", u8::from(healthy))?;

    writeln!(
        out,
        "# HELP blockfrost_gateway_connected_relays Number of relays currently connected via WebSocket."
    )?;
    writeln!(out, "# TYPE blockfrost_gateway_connected_relays gauge")?;
    writeln!(out, "blockfrost_gateway_connected_relays {}", relays.len())?;

    for (name, help, value) in [
        (
            "blockfrost_gateway_db_pool_max_size",
            "Maximum number of PostgreSQL connections in the pool.",
            db_pool.max_size,
        ),
        (
            "blockfrost_gateway_db_pool_size",
            "Current number of open PostgreSQL connections in the pool.",
            db_pool.size,
        ),
        (
            "blockfrost_gateway_db_pool_available",
            "Idle PostgreSQL connections available for immediate use.",
            db_pool.available,
        ),
        (
            "blockfrost_gateway_db_pool_waiting",
            "Tasks currently waiting for a PostgreSQL connection.",
            db_pool.waiting,
        ),
    ] {
        writeln!(out, "# HELP {name} {help}")?;
        writeln!(out, "# TYPE {name} gauge")?;
        writeln!(out, "{name} {value}")?;
    }

    for &(name, kind, help, value) in RELAY_METRICS {
        writeln!(out, "# HELP {name} {help}")?;
        writeln!(out, "# TYPE {name} {kind}")?;
        for r in &relays {
            if let Some(v) = value(r) {
                writeln!(out, "{name}{} {v}", relay_labels(r))?;
            }
        }
    }

    // The version metric needs an extra label, so it doesn’t fit `RELAY_METRICS`:
    {
        let name = "blockfrost_gateway_relay_info";
        writeln!(
            out,
            "# HELP {name} Version of the Platform run by the relay, in the `version` label (value is always 1)."
        )?;
        writeln!(out, "# TYPE {name} gauge")?;
        for r in &relays {
            if let Some(version) = &r.version {
                writeln!(
                    out,
                    "{name}{{relay=\"{}\",api_prefix=\"{}\",version=\"{}\"}} 1",
                    sanitize_label_value(&r.relay),
                    r.api_prefix,
                    sanitize_label_value(version),
                )?;
            }
        }
    }

    Ok(out)
}

pub async fn route(
    Extension(load_balancer): Extension<LoadBalancerState>,
    Extension(db): Extension<DB>,
    Extension(health_monitor): Extension<HealthMonitor>,
    Extension(prometheus_handle): Extension<PrometheusHandle>,
) -> Result<impl IntoResponse, StatusCode> {
    let healthy = health_monitor.current_status().await.healthy;
    let mut body = prometheus_handle.render();
    body.push_str(
        &render_prometheus(&load_balancer, &db.pool_status(), healthy)
            .await
            .map_err(|e| {
                error!("failed to render gateway metrics: {e}");
                StatusCode::INTERNAL_SERVER_ERROR
            })?,
    );
    Ok((
        [(
            axum::http::header::CONTENT_TYPE,
            "text/plain; version=0.0.4; charset=utf-8",
        )],
        body,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::load_balancer::{PlatformHealth, RelayState};
    use crate::types::AssetName;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::{Mutex, mpsc};

    fn test_key() -> [u8; 32] {
        *blake3::hash(b"test-peer-secret").as_bytes()
    }

    fn test_pool_status() -> PoolStatus {
        PoolStatus {
            max_size: 8,
            size: 3,
            available: 2,
            waiting: 1,
        }
    }

    fn test_relay_state(name: &str) -> RelayState {
        let (new_request_channel, _request_rx) = mpsc::channel(1);
        let (do_finish, _finish_rx) = mpsc::channel(1);

        RelayState {
            name: AssetName(name.to_string()),
            new_request_channel,
            do_finish,
            requests_in_progress: Arc::new(Mutex::new(HashMap::new())),
            network_rtt: Arc::new(Mutex::new(None)),
            connected_since: std::time::Instant::now(),
            requests_sent: Arc::new(atomic::AtomicU64::new(0)),
            responses_received: Arc::new(atomic::AtomicU64::new(0)),
            platform_health: Arc::new(Mutex::new(None)),
        }
    }

    #[tokio::test]
    async fn renders_prometheus_text() {
        let lb = LoadBalancerState::new(None, test_key());
        let uuid = Uuid::parse_str("513d26a9-9fea-4fbd-8ff4-d9ab00875c59").unwrap();
        let relay = test_relay_state("Icebreaker2");
        relay.requests_sent.store(5, atomic::Ordering::SeqCst);
        relay.responses_received.store(4, atomic::Ordering::SeqCst);
        *relay.platform_health.lock().await = Some(PlatformHealth {
            healthy: true,
            version: Some("1.2.3".to_string()),
            has_data_node: Some(false),
        });
        lb.active_relays.lock().await.insert(uuid, relay);

        let out = render_prometheus(&lb, &test_pool_status(), true)
            .await
            .expect("render metrics");

        assert!(out.contains("# TYPE blockfrost_gateway_healthy gauge"));
        assert!(out.contains("\nblockfrost_gateway_healthy 1\n"));
        assert!(out.contains("# TYPE blockfrost_gateway_connected_relays gauge"));
        assert!(out.contains("\nblockfrost_gateway_connected_relays 1\n"));
        assert!(out.contains("# TYPE blockfrost_gateway_db_pool_max_size gauge"));
        assert!(out.contains("\nblockfrost_gateway_db_pool_max_size 8\n"));
        assert!(out.contains("\nblockfrost_gateway_db_pool_size 3\n"));
        assert!(out.contains("\nblockfrost_gateway_db_pool_available 2\n"));
        assert!(out.contains("\nblockfrost_gateway_db_pool_waiting 1\n"));
        assert!(out.contains("# TYPE blockfrost_gateway_relay_requests_sent_total counter"));
        assert!(out.contains(
            "blockfrost_gateway_relay_requests_sent_total{relay=\"Icebreaker2\",api_prefix=\"513d26a9-9fea-4fbd-8ff4-d9ab00875c59\"} 5"
        ));
        assert!(out.contains(
            "blockfrost_gateway_relay_responses_received_total{relay=\"Icebreaker2\",api_prefix=\"513d26a9-9fea-4fbd-8ff4-d9ab00875c59\"} 4"
        ));
        assert!(out.contains(
            "blockfrost_gateway_relay_up{relay=\"Icebreaker2\",api_prefix=\"513d26a9-9fea-4fbd-8ff4-d9ab00875c59\"} 1"
        ));
        assert!(out.contains(
            "blockfrost_gateway_relay_healthy{relay=\"Icebreaker2\",api_prefix=\"513d26a9-9fea-4fbd-8ff4-d9ab00875c59\"} 1"
        ));
        assert!(out.contains(
            "blockfrost_gateway_relay_data_node_up{relay=\"Icebreaker2\",api_prefix=\"513d26a9-9fea-4fbd-8ff4-d9ab00875c59\"} 0"
        ));
        assert!(out.contains(
            "blockfrost_gateway_relay_info{relay=\"Icebreaker2\",api_prefix=\"513d26a9-9fea-4fbd-8ff4-d9ab00875c59\",version=\"1.2.3\"} 1"
        ));
        assert!(
            !out.contains("blockfrost_gateway_relay_network_rtt_seconds{relay=\"Icebreaker2\"")
        );
    }

    #[tokio::test]
    async fn omits_health_metrics_before_first_check() {
        let lb = LoadBalancerState::new(None, test_key());
        let uuid = Uuid::parse_str("00000000-0000-0000-0000-000000000002").unwrap();
        lb.active_relays
            .lock()
            .await
            .insert(uuid, test_relay_state("Icebreaker2"));

        let out = render_prometheus(&lb, &test_pool_status(), true)
            .await
            .expect("render metrics");

        assert!(out.contains("# TYPE blockfrost_gateway_relay_healthy gauge"));
        assert!(out.contains("# TYPE blockfrost_gateway_relay_info gauge"));
        assert!(!out.contains("blockfrost_gateway_relay_healthy{"));
        assert!(!out.contains("blockfrost_gateway_relay_data_node_up{"));
        assert!(!out.contains("blockfrost_gateway_relay_info{"));
    }

    #[test]
    fn recorder_renders_build_info() {
        let out = setup_metrics_recorder().render();
        assert!(out.contains("# TYPE blockfrost_gateway_build_info gauge"));
        assert!(out.contains(&format!("version=\"{}\"", env!("CARGO_PKG_VERSION"))));
        assert!(out.contains(&format!("revision=\"{}\"", env!("GIT_REVISION"))));
    }

    #[tokio::test]
    async fn reports_zero_when_no_relays() {
        let lb = LoadBalancerState::new(None, test_key());
        let out = render_prometheus(&lb, &test_pool_status(), true)
            .await
            .expect("render metrics");
        assert!(out.contains("\nblockfrost_gateway_connected_relays 0\n"));
    }

    #[tokio::test]
    async fn reports_unhealthy_gateway() {
        let lb = LoadBalancerState::new(None, test_key());
        let out = render_prometheus(&lb, &test_pool_status(), false)
            .await
            .expect("render metrics");
        assert!(out.contains("\nblockfrost_gateway_healthy 0\n"));
    }

    #[tokio::test]
    async fn escapes_label_values() {
        let lb = LoadBalancerState::new(None, test_key());
        let uuid = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        lb.active_relays
            .lock()
            .await
            .insert(uuid, test_relay_state("we\"ird\\name"));

        let out = render_prometheus(&lb, &test_pool_status(), true)
            .await
            .expect("render metrics");
        assert!(out.contains("relay=\"we\\\"ird\\\\name\""));
    }
}
