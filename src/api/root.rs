use crate::{node::sync_progress::NodeInfo, BlockfrostError, NodePool};
use axum::{response::IntoResponse, Extension, Json};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RootResponse {
    pub name: String,
    pub version: String,
    pub healthy: bool,
    pub node_info: NodeInfo,
    pub errors: Vec<String>,
}

pub async fn route(
    Extension(node): Extension<NodePool>,
) -> Result<impl IntoResponse, BlockfrostError> {
    let errors = vec![];

    // XXX: Axum must not abort Ouroboros protocols in the middle, hence a separate Tokio task:
    let node_info: NodeInfo = tokio::spawn(async move {
        let mut node = node.get().await?;
        node.sync_progress().await
    })
    .await
    .expect("sync_progress panic!")?;

    let response = RootResponse {
        name: env!("CARGO_PKG_NAME").to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        node_info,
        healthy: errors.is_empty(),
        errors,
    };

    Ok(Json(response))
}
