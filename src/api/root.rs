use crate::{node::sync_progress::NodeInfo, BlockfrostError, NodePool};
use axum::{response::IntoResponse, Extension, Json};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct RootResponse {
    pub name: String,
    pub version: String,
    pub healthy: bool,
    #[serde(rename = "nodeInfo")]
    pub node_info: NodeInfo,
    pub errors: Vec<String>,
}

pub async fn route(
    Extension(node): Extension<NodePool>,
) -> Result<impl IntoResponse, BlockfrostError> {
    let errors = vec![];
    let mut node = node.get().await?;
    let node_info = node.sync_progress().await?;

    let response = RootResponse {
        name: "blockfrost-platform".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        node_info,
        healthy: errors.is_empty(),
        errors,
    };

    Ok(Json(response))
}
