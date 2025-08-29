use crate::config::Config;
use axum::{Extension, Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct Response {
    pub url: Option<String>,
    pub version: String,
    pub healthy: bool,
    pub commit: &'static str,
}

pub async fn route(Extension(config): Extension<Config>) -> Json<Response> {
    let response = Response {
        url: config.server.url.clone(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        commit: env!("GIT_REVISION"),
        healthy: true,
    };

    Json(response)
}
