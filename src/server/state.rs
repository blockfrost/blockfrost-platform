use api_provider::{api::Api, types::GenesisResponse};
use axum::extract::State;
use common::{config::Config, types::Network};
use std::sync::Arc;

#[derive(Clone)]
pub struct AppState {
    pub config: Arc<Config>,
    pub genesis: Arc<Vec<(Network, GenesisResponse)>>,
    pub api: Arc<Api>,
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
