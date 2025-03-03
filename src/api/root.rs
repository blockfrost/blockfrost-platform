use crate::config::Config;
use axum::{Extension, Json};
use serde::Serialize;

#[derive(Serialize)]
pub struct Response {
    pub url: String,
    pub version: String,
    pub healthy: bool,
    pub commit: &'static str,
}

pub async fn route(Extension(config): Extension<Config>) -> Json<Response> {
    let is_dev = config.blockfrost.project_id.contains("preview");

    let url = if is_dev {
        "https://api-dev.icebreakers.blockfrost.io/"
    } else {
        "https://icebreakers-api.blockfrost.io"
    }
    .to_string();

    let response = Response {
        url,
        version: env!("CARGO_PKG_VERSION").to_string(),
        commit: env!("GIT_REVISION"),
        healthy: true,
    };

    Json(response)
}
