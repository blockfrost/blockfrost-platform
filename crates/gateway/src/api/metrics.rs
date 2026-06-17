use crate::load_balancer::LoadBalancerState;
use axum::{Extension, http::StatusCode, response::IntoResponse};
use std::fmt::Write as _;
use std::sync::atomic;
use tracing::error;
use uuid::Uuid;

struct RelayMetrics {
    relay: String,
    api_prefix: Uuid,
    network_rtt_seconds: Option<f64>,
    connected_since_seconds: i64,
    requests_sent: u64,
    responses_received: u64,
    requests_in_progress: u64,
}

fn escape_label_value(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

fn relay_labels(r: &RelayMetrics) -> String {
    format!(
        "{{relay=\"{}\",api_prefix=\"{}\"}}",
        escape_label_value(&r.relay),
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
        "Total requests forwarded to the relay since it connected.",
        |r| Some(r.requests_sent.to_string()),
    ),
    (
        "blockfrost_gateway_relay_responses_received_total",
        "counter",
        "Total responses received from the relay since it connected.",
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
) -> Result<String, std::fmt::Error> {
    let now_chrono = chrono::Utc::now();
    let now_instant = std::time::Instant::now();

    let mut relays: Vec<RelayMetrics> = Vec::new();
    for (api_prefix, relay_state) in load_balancer.active_relays.lock().await.iter() {
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
        });
    }

    let mut out = String::new();

    writeln!(
        out,
        "# HELP blockfrost_gateway_connected_relays Number of relays currently connected via WebSocket."
    )?;
    writeln!(out, "# TYPE blockfrost_gateway_connected_relays gauge")?;
    writeln!(out, "blockfrost_gateway_connected_relays {}", relays.len())?;

    for &(name, kind, help, value) in RELAY_METRICS {
        writeln!(out, "# HELP {name} {help}")?;
        writeln!(out, "# TYPE {name} {kind}")?;
        for r in &relays {
            if let Some(v) = value(r) {
                writeln!(out, "{name}{} {v}", relay_labels(r))?;
            }
        }
    }

    Ok(out)
}

pub async fn route(
    Extension(load_balancer): Extension<LoadBalancerState>,
) -> Result<impl IntoResponse, StatusCode> {
    let body = render_prometheus(&load_balancer).await.map_err(|e| {
        error!("failed to render gateway metrics: {e}");
        StatusCode::INTERNAL_SERVER_ERROR
    })?;
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
    use crate::load_balancer::RelayState;
    use crate::types::AssetName;
    use std::collections::HashMap;
    use std::sync::Arc;
    use tokio::sync::{Mutex, mpsc};

    fn test_key() -> [u8; 32] {
        *blake3::hash(b"test-peer-secret").as_bytes()
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
        }
    }

    #[tokio::test]
    async fn renders_prometheus_text() {
        let lb = LoadBalancerState::new(None, test_key());
        let uuid = Uuid::parse_str("513d26a9-9fea-4fbd-8ff4-d9ab00875c59").unwrap();
        let relay = test_relay_state("Icebreaker2");
        relay.requests_sent.store(5, atomic::Ordering::SeqCst);
        relay.responses_received.store(4, atomic::Ordering::SeqCst);
        lb.active_relays.lock().await.insert(uuid, relay);

        let out = render_prometheus(&lb).await.expect("render metrics");

        assert!(out.contains("# TYPE blockfrost_gateway_connected_relays gauge"));
        assert!(out.contains("\nblockfrost_gateway_connected_relays 1\n"));
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
        assert!(
            !out.contains("blockfrost_gateway_relay_network_rtt_seconds{relay=\"Icebreaker2\"")
        );
    }

    #[tokio::test]
    async fn reports_zero_when_no_relays() {
        let lb = LoadBalancerState::new(None, test_key());
        let out = render_prometheus(&lb).await.expect("render metrics");
        assert!(out.contains("\nblockfrost_gateway_connected_relays 0\n"));
    }

    #[tokio::test]
    async fn escapes_label_values() {
        let lb = LoadBalancerState::new(None, test_key());
        let uuid = Uuid::parse_str("00000000-0000-0000-0000-000000000001").unwrap();
        lb.active_relays
            .lock()
            .await
            .insert(uuid, test_relay_state("we\"ird\\name"));

        let out = render_prometheus(&lb).await.expect("render metrics");
        assert!(out.contains("relay=\"we\\\"ird\\\\name\""));
    }
}
