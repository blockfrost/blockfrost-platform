use api_provider::types::GenesisResponse;
use axum::extract::State;
use common::{config::Config, errors::BlockfrostError, types::Network};
use dolos::client::Dolos;
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub genesis: Arc<Vec<(Network, GenesisResponse)>>,
    pub dolos: Option<Dolos>,
}

impl AppState {
    pub fn get_dolos(&self) -> Result<&Dolos, BlockfrostError> {
        self.dolos.as_ref().ok_or_else(|| {
            BlockfrostError::internal_server_error("Dolos is not configured".to_string())
        })
    }
}

pub type AppStateExt = State<AppState>;

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ApiPrefix(pub Option<uuid::Uuid>);

impl std::fmt::Display for ApiPrefix {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Some(u) => write!(f, "/{u}"),
            None => write!(f, "/"),
        }
    }
}
