use crate::blockfrost::BlockfrostAPI;
use crate::config::Config;
use crate::db::DB;
use crate::errors::APIError;
use crate::models::RequestNewItem;
use crate::payload::Payload;
use axum::body::Bytes;
use axum::http::{HeaderMap, HeaderValue};
use axum::{Extension, Json};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseSuccess {
    route: String,
    status: String,
}

pub async fn route(
    Extension(db): Extension<DB>,
    Extension(config): Extension<Config>,
    Extension(blockfrost_api): Extension<BlockfrostAPI>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<Json<ResponseSuccess>, APIError> {
    let payload: Payload = match serde_json::from_slice(&body) {
        Ok(payload) => payload,
        Err(e) => return Err(APIError::Validaion(e.to_string())),
    };

    // validate POST payload
    Payload::validate(&payload)?;

    // check if user has correct secret
    let authorized_user = db.authorize_user(payload.secret).await?;

    // get IP address
    let ip_address: &str = headers
        .get("x-forwarded-for")
        .and_then(|val: &HeaderValue| val.to_str().ok())
        .unwrap_or("unknown");

    // check if the server is accessible
    if !is_accessible(ip_address, payload.port).await {
        return Err(APIError::NotAccessible());
    }

    // check if NFT is at the address
    blockfrost_api
        .nft_exists(&payload.reward_address, &config.blockfrost.nft_asset)
        .await
        .map_err(|_| APIError::License(payload.reward_address.clone()))?;

    let new_item_request = RequestNewItem {
        user_id: authorized_user.user_id,
        route: Uuid::new_v4().to_string(),
        mode: payload.mode.clone(),
        ip_address: ip_address.to_string(),
        port: payload.port,
        reward_address: payload.reward_address.clone(),
    };

    let success_response = ResponseSuccess {
        status: "registered".to_string(),
        route: format!("/{}", new_item_request.route.to_string()),
    };

    db.insert_request(new_item_request).await?;

    Ok(Json(success_response))
}

async fn is_accessible(ip: &str, port: i32) -> bool {
    let client = Client::new();
    let url = format!("http://{}:{}", ip, port);
    let request_future = client.get(&url).send();

    match timeout(Duration::from_secs(5), request_future).await {
        Ok(Ok(response)) => response.status().is_success(),
        _ => false,
    }
}
