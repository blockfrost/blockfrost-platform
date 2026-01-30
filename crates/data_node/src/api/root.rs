use crate::client::DataNode;
use axum::Json;
use bf_common::errors::BlockfrostError;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DataNodeRootResponse {
    pub url: String,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub revision: Option<String>,
}

impl DataNode {
    pub async fn root(&self) -> Result<Json<DataNodeRootResponse>, BlockfrostError> {
        self.client.get("", None).await
    }
}
