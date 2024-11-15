use crate::{
    errors::BlockfrostError,
    node::{pool::NodePool, sync::SyncProgress},
};
use axum::{response::IntoResponse, Extension, Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct Response {
    pub name: String,
    pub version: String,
    pub sync_progress: SyncProgress,
    pub healthy: bool,
    pub errors: Vec<String>,
}

pub async fn route(
    Extension(node): Extension<NodePool>,
) -> Result<impl IntoResponse, BlockfrostError> {
    let errors = vec![];
    let mut node = node.get().await?;
    let sync_progress = node.sync_progress().await?;

    let response = Response {
        name: "blockfrost-platform".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        sync_progress,
        healthy: errors.is_empty(),
        errors,
    };

    Ok(Json(response))
}
