use crate::blockfrost::BlockfrostAPI;
use crate::config::Config;
use crate::db::DB;
use crate::errors::APIError;
use crate::models::RequestNewItem;
use crate::payload::Payload;
use axum::body::Bytes;
use axum::http::HeaderMap;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;

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
    let ip_header_value = headers
        .get("HTTP_DO_CONNECTING_IP")
        .or_else(|| headers.get("X-Forwarded-For"))
        .and_then(|val| val.to_str().ok())
        .unwrap_or("unknown");

    let ip_address: &str = ip_header_value.split(',').next().unwrap_or("unknown").trim();

    // check if the server is accessible
    if !is_port_open(ip_address, payload.port).await {
        return Err(APIError::NotAccessible {
            ip: ip_address.to_string(),
            port: payload.port,
        });
    }

    // check if NFT is at the address
    let asset = blockfrost_api
        .nft_exists(&payload.reward_address, &config.blockfrost.nft_asset)
        .await
        .map_err(|_| APIError::License(payload.reward_address.clone()))?;

    let new_item_request = RequestNewItem {
        user_id: authorized_user.user_id,
        mode: payload.mode.clone(),
        ip_address: ip_address.to_string(),
        port: payload.port,
        route: payload.api_prefix.clone(),
        reward_address: payload.reward_address.clone(),
        asset_name: Some(asset.asset_name),
    };

    let success_response = ResponseSuccess {
        status: "registered".to_string(),
        route: payload.api_prefix.clone(),
    };

    db.insert_request(new_item_request).await?;

    Ok(Json(success_response))
}

async fn is_port_open(ip: &str, port: i32) -> bool {
    let address = format!("{}:{}", ip, port);
    let connection_future = TcpStream::connect(&address);
    matches!(
        timeout(Duration::from_secs(10), connection_future).await,
        Ok(Ok(_stream))
    )
}
