use axum::Extension;
use axum::Json;
use serde::Serialize;
use std::sync::Arc;
use tokio::sync::RwLock;

use crate::node::Node;

#[derive(Serialize)]
pub struct Response {
    pub name: String,
    pub version: String,
    pub node_version: String,
    pub errors: Option<Vec<String>>,
    pub healthy: bool,
}

pub async fn route(Extension(node): Extension<Arc<RwLock<Node>>>) -> Json<Response> {
    let node_version_result = {
        let mut node = node.write().await;
        node.version().await
    };

    let (node_version, errors) = match node_version_result {
        Ok(version) => (version, None),
        Err(error) => (
            "unknown".to_string(),
            Some(vec![format!("Failed to determine node version: {}", error)]),
        ),
    };

    let response = Response {
        name: "blockfrost-platform".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        node_version,
        healthy: errors.is_none(),
        errors,
    };

    Json(response)
}

