use crate::blockfrost::BlockfrostAPI;
use crate::config::Config;
use crate::db::DB;
use crate::errors::APIError;
use crate::models::RequestNewItem;
use crate::payload::Payload;
use axum::body::Bytes;
use axum::extract::ConnectInfo;
use axum::{Extension, Json};
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
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
    ConnectInfo(ip_address): ConnectInfo<SocketAddr>,
    body: Bytes,
) -> Result<Json<ResponseSuccess>, APIError> {
    let payload: Payload = match serde_json::from_slice(&body) {
        Ok(payload) => payload,
        Err(e) => return Err(APIError::Validaion(e.to_string())),
    };

    // validate POST payload
    Payload::validate(&payload)?;

    // check if user has correct secret
    db.authorize_user(payload.secret).await?;

    // check if the server is accessible
    // let url = config
    //     .blockfrost
    //     .api_url_pattern
    //     .replace("{IP}", &ip_address.to_string())
    //     .replace("{PORT}", &payload.port.to_string());

    // reqwest::get(&url).await.map_err(|_| APIError::NotAccessible())?;

    // check if NFT is at the address
    blockfrost_api
        .nft_exists(&payload.reward_address, &config.blockfrost.nft_asset)
        .await
        .map_err(|_| APIError::License(payload.reward_address.clone()))?;

    let new_item_request = RequestNewItem {
        route: Uuid::new_v4().to_string(),
        mode: payload.mode.clone(),
        ip_address: ip_address.to_string(),
        port: payload.port.clone(),
        reward_address: payload.reward_address.clone(),
    };

    db.insert_request(new_item_request).await?;

    let success_response = ResponseSuccess {
        status: "registered".to_string(),
        route: "URL_PLACEHOLDET".to_string(),
    };

    Ok(Json(success_response))
}
