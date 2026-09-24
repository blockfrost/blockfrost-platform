use crate::config::Config;
use crate::health_monitor::HealthMonitor;
use axum::{Extension, Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;

#[derive(Serialize)]
pub struct Response {
    pub url: Option<url::Url>,
    pub version: String,
    pub healthy: bool,
    pub commit: &'static str,
    pub errors: Vec<&'static str>,
}

pub async fn route(
    Extension(config): Extension<Config>,
    Extension(health_monitor): Extension<HealthMonitor>,
) -> impl IntoResponse {
    let status = health_monitor.current_status().await;

    let http_status = if status.healthy {
        StatusCode::OK
    } else {
        StatusCode::SERVICE_UNAVAILABLE
    };

    let response = Response {
        url: config.server.url.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        commit: env!("GIT_REVISION"),
        healthy: status.healthy,
        errors: status.errors.iter().map(|e| e.code).collect(),
    };

    (http_status, Json(response))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::health_monitor::{HealthError, HealthStatus};
    use crate::types::Network;

    fn test_config() -> Config {
        Config {
            server: crate::config::Server {
                address: "127.0.0.1:0".to_string(),
                log_level: tracing::Level::INFO,
                network: Network::Preview,
                url: Some(url::Url::parse("https://gateway.example.com").unwrap()),
                peer_urls: vec![],
                peer_secret: [7; 32],
            },
            database: crate::config::Db {
                connection_string: "postgresql://user:pass@localhost/db".to_string(),
                pool_max_size: std::num::NonZeroUsize::new(8).unwrap(),
            },
            blockfrost: crate::config::Blockfrost {
                project_id: "previewXXX".to_string(),
                nft_asset: "asset".to_string(),
            },
            hydra_platform: None,
            hydra_bridge: None,
        }
    }

    async fn get_root(status: HealthStatus) -> (StatusCode, serde_json::Value) {
        let monitor = HealthMonitor::new_static(status);
        let response = route(Extension(test_config()), Extension(monitor))
            .await
            .into_response();
        let http_status = response.status();
        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .expect("read body");
        (http_status, serde_json::from_slice(&body).expect("JSON"))
    }

    #[tokio::test]
    async fn healthy_returns_200() {
        let (status, json) = get_root(HealthStatus {
            healthy: true,
            errors: vec![],
        })
        .await;

        assert_eq!(status, StatusCode::OK);
        assert_eq!(json["healthy"], serde_json::json!(true));
        assert_eq!(json["errors"], serde_json::json!([]));
    }

    #[tokio::test]
    async fn unhealthy_returns_503_with_error_codes_only() {
        let detail = "database pool error: connection to internal-db-host refused";
        let (status, json) = get_root(HealthStatus {
            healthy: false,
            errors: vec![HealthError {
                code: "database_unreachable",
                detail: detail.to_string(),
            }],
        })
        .await;

        assert_eq!(status, StatusCode::SERVICE_UNAVAILABLE);
        assert_eq!(json["healthy"], serde_json::json!(false));
        assert_eq!(json["errors"], serde_json::json!(["database_unreachable"]));
        // The detailed reason must not leak into the public response.
        assert!(!json.to_string().contains("internal-db-host"));
    }
}
