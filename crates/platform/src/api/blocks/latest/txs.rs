use crate::{api::ApiResult, server::state::AppState};
use axum::extract::{Query, State};
use bf_common::pagination::{Pagination, PaginationQuery};

pub async fn route(
    State(state): State<AppState>,
    Query(pagination_query): Query<PaginationQuery>,
) -> ApiResult<Vec<String>> {
    let pagination = Pagination::from_query(pagination_query)?;
    let data_node = state.data_node()?;

    data_node.blocks().latest_txs(&pagination).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, Mode};
    use axum::{
        Json, Router,
        body::{Body, to_bytes},
        http::{Request, StatusCode},
        routing::get,
    };
    use bf_common::types::{LogLevel, Network};
    use bf_data_node::client::DataNode;
    use std::{collections::HashMap, sync::Arc, time::Duration};
    use tokio::{sync::mpsc, task::JoinHandle};
    use tower::ServiceExt;

    async fn mock_app() -> (
        Router,
        mpsc::Receiver<HashMap<String, String>>,
        JoinHandle<()>,
    ) {
        let (query_tx, query_rx) = mpsc::channel(16);
        let backend = Router::new().route(
            "/blocks/latest/txs",
            get(move |Query(query): Query<HashMap<String, String>>| {
                let query_tx = query_tx.clone();
                async move {
                    query_tx.send(query.clone()).await.unwrap();
                    let count: usize = query
                        .get("count")
                        .map(|v| v.parse().unwrap())
                        .unwrap_or(100);
                    let page: usize = query.get("page").map(|v| v.parse().unwrap()).unwrap_or(1);
                    let mut txs = vec![
                        "a".repeat(64),
                        "b".repeat(64),
                        "c".repeat(64),
                        "d".repeat(64),
                    ];
                    if query.get("order").is_some_and(|v| v == "desc") {
                        txs.reverse();
                    }
                    Json(
                        txs.into_iter()
                            .skip((page - 1) * count)
                            .take(count)
                            .collect::<Vec<_>>(),
                    )
                }
            }),
        );
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let endpoint = format!("http://{}/", listener.local_addr().unwrap());
        let server = tokio::spawn(async move {
            axum::serve(listener, backend).await.unwrap();
        });
        let state = AppState {
            config: Arc::new(Config {
                server_address: "127.0.0.1".parse().unwrap(),
                server_port: 0,
                server_concurrency_limit: 2048,
                max_response_body_bytes: bf_common::DEFAULT_MAX_BODY_BYTES,
                log_level: LogLevel::Info.into(),
                node_socket_path: String::new(),
                mode: Mode::Compact,
                icebreakers_config: None,
                max_pool_connections: 1,
                no_metrics: true,
                network: Network::Preview,
                custom_genesis_config: None,
                genesis: vec![],
                data_node: None,
                hydra: None,
            }),
            data_node: Some(DataNode::new(&endpoint, Duration::from_secs(2)).unwrap()),
        };
        let app = Router::new()
            .route("/blocks/latest/txs", get(route))
            .with_state(state);
        (app, query_rx, server)
    }

    #[tokio::test]
    async fn forwards_pagination_and_returns_requested_transactions() {
        let (app, mut queries, server) = mock_app().await;
        for (query, expected, count, page, order) in [
            ("?count=1&page=2&order=desc", vec!["c"], "1", "2", "desc"),
            ("?count=2&page=2&order=asc", vec!["c", "d"], "2", "2", "asc"),
            ("?count=2&page=3", vec![], "2", "3", "asc"),
            ("", vec!["a", "b", "c", "d"], "100", "1", "asc"),
        ] {
            let request = Request::builder()
                .uri(format!("/blocks/latest/txs{query}"))
                .body(Body::empty())
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::OK);
            let body = to_bytes(response.into_body(), usize::MAX).await.unwrap();
            assert_eq!(
                serde_json::from_slice::<Vec<String>>(&body).unwrap(),
                expected
                    .into_iter()
                    .map(|s| s.repeat(64))
                    .collect::<Vec<_>>()
            );
            let forwarded = queries.recv().await.unwrap();
            assert_eq!(forwarded.get("count").unwrap(), count);
            assert_eq!(forwarded.get("page").unwrap(), page);
            assert_eq!(forwarded.get("order").unwrap(), order);
        }
        server.abort();
    }

    #[tokio::test]
    async fn rejects_invalid_pagination_before_contacting_backend() {
        let (app, mut queries, server) = mock_app().await;
        for query in [
            "count=0",
            "count=101",
            "count=invalid",
            "page=0",
            "page=21474837",
            "page=invalid",
            "order=invalid",
        ] {
            let request = Request::builder()
                .uri(format!("/blocks/latest/txs?{query}"))
                .body(Body::empty())
                .unwrap();
            let response = app.clone().oneshot(request).await.unwrap();
            assert_eq!(response.status(), StatusCode::BAD_REQUEST, "{query}");
        }
        assert!(matches!(
            queries.try_recv(),
            Err(mpsc::error::TryRecvError::Empty)
        ));
        server.abort();
    }
}
