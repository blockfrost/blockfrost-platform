use axum::{Json, Router, routing::get};
use bf_api_provider::types::HealthResponse;
use bf_data_node::api::root::DataNodeRootResponse;

pub struct MockDataNode {
    pub url: String,
}

impl MockDataNode {
    pub async fn healthy() -> Self {
        Self::start(true, Some("test-revision".to_string())).await
    }

    pub async fn unhealthy() -> Self {
        Self::start(false, None).await
    }

    async fn start(is_healthy: bool, revision: Option<String>) -> Self {
        let mock_app = Router::new()
            .route(
                "/",
                get(move || {
                    let revision = revision.clone();
                    async move {
                        Json(DataNodeRootResponse {
                            url: "http://this.is.a.test.url".to_string(),
                            version: "0.0.0-test".to_string(),
                            revision,
                        })
                    }
                }),
            )
            .route(
                "/health",
                get(move || async move { Json(HealthResponse { is_healthy }) }),
            );

        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        let url = format!("http://{addr}");

        tokio::spawn(async move {
            axum::serve(listener, mock_app).await.unwrap();
        });

        tokio::time::sleep(tokio::time::Duration::from_millis(50)).await;

        Self { url }
    }

    pub fn unreachable() -> Self {
        Self {
            url: "http://127.0.0.1:1".to_string(),
        }
    }
}
