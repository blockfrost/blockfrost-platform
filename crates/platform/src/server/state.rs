use crate::config::Config;
use axum::extract::State;
use bf_common::errors::BlockfrostError;
use bf_data_node::client::DataNode;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub data_node: Option<DataNode>,
}

impl AppState {
    pub fn data_node(&self) -> Result<&DataNode, BlockfrostError> {
        self.data_node.as_ref().ok_or_else(|| {
            BlockfrostError::internal_server_error("Data node is not configured".to_string())
        })
    }
}

pub type AppStateExt = State<AppState>;
