use axum::Json;
use serde::Serialize;

#[derive(Serialize)]
pub struct Response {
    pub url: String,
    pub version: String,
    pub healthy: bool,
    pub commit: &'static str,
}

pub async fn route() -> Json<Response> {
    let response = Response {
        url: "https://icebreakers.blockfrost.io".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        commit: env!("GIT_REVISION"),
        healthy: true,
    };

    Json(response)
}
