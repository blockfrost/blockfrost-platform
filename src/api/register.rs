use crate::blockfrost::BlockfrostAPI;
use crate::config::Config;
use crate::errors::APIError;
use crate::payload::Payload;
use crate::{
    models::{Request, RequestNewItem},
    schema,
};
use axum::body::Bytes;
use axum::extract::ConnectInfo;
use axum::{Extension, Json};
use deadpool_diesel::postgres::Pool;
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use std::net::SocketAddr;
use uuid::Uuid;

#[derive(Serialize, Deserialize, Debug)]
pub struct ResponseSuccess {
    route: String,
    status: String,
}

pub async fn route(
    Extension(pool): Extension<Pool>,
    Extension(config): Extension<Config>,
    Extension(blockfrost_api): Extension<BlockfrostAPI>,
    ConnectInfo(ip_address): ConnectInfo<SocketAddr>,
    body: Bytes,
) -> Result<Json<ResponseSuccess>, APIError> {
    let payload: Payload = match serde_json::from_slice(&body) {
        Ok(payload) => payload,
        Err(e) => return Err(APIError::ValidationError(e.to_string())),
    };

    // validate POST payload
    Payload::validate(&payload)?;

    // check if the server is accessible
    let url = config
        .blockfrost
        .api_url_pattern
        .replace("{IP}", &ip_address.to_string())
        .replace("{PORT}", &payload.port.to_string());

    reqwest::get(&url)
        .await
        .map_err(|_| APIError::NotAccessible())?;

    // check if NFT is at the address
    blockfrost_api
        .nft_exists(&payload.reward_address, &config.blockfrost.nft_asset)
        .await
        .map_err(|_| APIError::LicenseError(payload.reward_address.clone()))?;

    // success -> save the request to the database
    let db_pool = pool.get().await.map_err(|_| APIError::UnexpectedError())?;

    let new_item_request = RequestNewItem {
        user_id: Uuid::new_v4().to_string(),
        mode: payload.mode.clone(),
        ip_address: ip_address.to_string(),
        port: payload.port.clone(),
        reward_address: payload.reward_address.clone(),
    };

    db_pool
        .interact(|db_pool| {
            diesel::insert_into(schema::requests::table)
                .values(new_item_request)
                .returning(Request::as_returning())
                .get_result(db_pool)
        })
        .await
        .map_err(|_| APIError::UnexpectedError())?
        .map_err(|_| APIError::UnexpectedError())?;

    let success_response = ResponseSuccess {
        status: "registered".to_string(),
        route: url,
    };

    Ok(Json(success_response))
}
